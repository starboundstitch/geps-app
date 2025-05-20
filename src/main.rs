use iced::widget::{
    button, column, container, horizontal_space, pick_list, rich_text, row, span, text, text_input,
    toggler, Container,
};
use iced::{font, Element, Fill, Length, Subscription, Task, Theme};

use plotters_iced::{Chart, ChartBuilder, ChartWidget, DrawingBackend};
use std::collections::VecDeque;
use std::fs::File;

use pmbus_types_rs::{slinear11, ulinear16};

use iced::time::{self, Duration, Instant};

use chrono::{DateTime, Utc};

const LIGHT_THEME: Theme = Theme::CatppuccinLatte;
const DARK_THEME: Theme = Theme::CatppuccinFrappe;

pub fn main() -> iced::Result {
    let _app = App::default();
    iced::application("GEPS App", App::update, App::view)
        .theme(theme)
        .subscription(App::subscription)
        .antialiasing(true)
        .run()
}

impl App {
    fn view(&self) -> Container<Message> {
        // Serial Ports
        let ports = serialport::available_ports().expect("No ports found!");
        let mut port_names: Vec<String> = vec![];
        for p in ports {
            port_names.push(p.port_name);
        }

        container(
            column![
                row![
                    rich_text![span("GEPS-App")
                        .font(font::Font::with_name("Noto Sans"))
                        .size(24)],
                    horizontal_space(),
                    pick_list(
                        port_names,
                        self.selected_port.clone(),
                        Message::PortSelected
                    )
                    .placeholder("Select Serial Port..."),
                    button("O").on_press(Message::PortOpen), // 󱘖
                    toggler(self.theme == DARK_THEME)
                        .label(if self.theme == DARK_THEME {
                            "☽"
                        } else {
                            "𖤓"
                        })
                        .text_shaping(text::Shaping::Advanced)
                        .text_size(24)
                        .on_toggle(Message::ThemeSwitch),
                ]
                .spacing(10),
                row![
                    column![
                        text("").size(12),
                        text("Voltage Set   (V):"),
                        text("Current Limit (A):"),
                    ]
                    .spacing(20)
                    .padding(10),
                    column![
                        text("VCORE"),
                        text_input("Voltage Setpoint...", &self.core_set)
                            .on_input(Message::VcoreVoltageUpdate)
                            .on_submit(Message::VcoreSetpointSubmit),
                        text_input("Current Limit...", &self.core_lim)
                            .on_input(Message::VcoreCurrentUpdate)
                            .on_submit(Message::VcoreCurrentSubmit),
                    ]
                    .spacing(10)
                    .padding(10),
                    column![
                        text("VMEM"),
                        text_input("Voltage Setpoint...", &self.mem_set)
                            .on_input(Message::VmemVoltageUpdate)
                            .on_submit(Message::VmemSetpointSubmit),
                        text_input("Current Limit...", &self.mem_lim)
                            .on_input(Message::VmemCurrentUpdate)
                            .on_submit(Message::VmemCurrentSubmit)
                    ]
                    .spacing(10)
                    .padding(10),
                ],
                self.chart.view(),
                row![
                    horizontal_space(),
                    button("Collect Data").on_press(Message::CollectData),
                ],
            ]
            .spacing(10),
        )
        .padding(10)
        .center_x(Fill)
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(100)).map(Message::Update)
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::ThemeSwitch(checked) => {
                if checked {
                    self.theme = DARK_THEME;
                } else {
                    self.theme = LIGHT_THEME;
                }
                self.chart.theme = self.theme.clone();
            }
            Message::CollectData => {
                if self.data_collect_time == -1 {
                    return Task::perform(
                        rfd::AsyncFileDialog::new()
                            .add_filter("csv", &["txt", "csv"])
                            .set_directory(std::env::current_dir().unwrap())
                            .save_file(),
                        Message::StartCollectData,
                    );
                } else {
                    self.stop_data();
                }
            }
            Message::StartCollectData(picked_file) => {
                if self.data_collect_time == -1 {
                    // Get the actual PATH
                    let file = match picked_file {
                        Some(file) => file,
                        None => return Task::none(),
                    };

                    self.data_collect_file = match File::create(file.path()) {
                        Err(why) => {
                            println!("Failed to Create File: {}", why);
                            return Task::none();
                        }
                        Ok(file) => Some(file),
                    };

                    // Write Header Line
                    let mut wtr =
                        csv::Writer::from_writer(self.data_collect_file.as_ref().unwrap());
                    wtr.write_record(&[
                        "Vcore Voltage".to_string(),
                        "Vcore Voltage Setpoint".to_string(),
                        "Vcore Current".to_string(),
                        "Vcore Current Limit".to_string(),
                        "Vcore Temperature".to_string(),
                        "Vmem Voltage".to_string(),
                        "Vmem Voltage Setpoint".to_string(),
                        "Vmem Current".to_string(),
                        "Vmem Current Limit".to_string(),
                        "Vmem Temperature".to_string(),
                    ])
                    .expect("File Error:");

                    self.data_collect_time = 0;

                    println!("Starting Data Collection");
                } else {
                    self.stop_data();
                }
            }
            Message::Update(_time) => {
                // Serial Bincode BS
                let mut serial_buf: Vec<u8> = vec![0; 128];
                let has_data;
                match &mut self.serial_port {
                    Some(val) => match val.read(serial_buf.as_mut_slice()) {
                        Ok(_val) => has_data = true,
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => has_data = false,
                        Err(_val) => has_data = false,
                    },
                    None => has_data = false,
                }

                if has_data {
                    // Get Device Struct
                    let decoded =
                        bincode::decode_from_slice(&serial_buf, bincode::config::standard());

                    let decoded: Device = match decoded {
                        Ok(val) => val.0,
                        Err(val) => {
                            println!("Error: {}", val);
                            Device::default()
                        }
                    };

                    self.device = decoded;
                }

                //** Update Chart From Values
                self.chart
                    .data_points
                    .push_front((Utc::now(), self.device.mem.voltage));

                // Check if logging and then output data from here
                if self.data_collect_time < 0 {
                    // Collect Data if in Collection Time
                } else if self.data_collect_time < 10 * 60 {
                    let mut wtr =
                        csv::Writer::from_writer(self.data_collect_file.as_ref().unwrap());
                    wtr.write_record(&[
                        self.device.core.voltage.to_string(),
                        self.device.core.voltage_setpoint.to_string(),
                        self.device.core.current.to_string(),
                        self.device.core.current_limit.to_string(),
                        self.device.core.temperature.to_string(),
                        self.device.mem.voltage.to_string(),
                        self.device.mem.voltage_setpoint.to_string(),
                        self.device.mem.current.to_string(),
                        self.device.mem.current_limit.to_string(),
                        self.device.mem.temperature.to_string(),
                    ])
                    .expect("File Error:");
                    self.data_collect_time += 1;
                } else {
                    self.stop_data();
                }
            }
            Message::PortSelected(val) => {
                self.selected_port = Some(val);
            }
            Message::PortOpen => {
                let port = serialport::new(self.selected_port.clone().unwrap(), 9600)
                    .timeout(Duration::from_millis(10))
                    .open()
                    .expect("Failed to Open Port");
                self.serial_port = Some(port);
            }
            Message::VcoreVoltageUpdate(val) => {
                if val.is_empty() || val.parse::<f32>().is_ok() {
                    self.core_set = val.clone();
                }
            }
            Message::VcoreCurrentUpdate(val) => {
                if val.is_empty() || val.parse::<f32>().is_ok() {
                    self.core_lim = val.clone();
                }
            }
            Message::VmemVoltageUpdate(val) => {
                if val.is_empty() || val.parse::<f32>().is_ok() {
                    self.mem_set = val.clone();
                }
            }
            Message::VmemCurrentUpdate(val) => {
                if val.is_empty() || val.parse::<f32>().is_ok() {
                    self.mem_lim = val.clone();
                }
            }
            Message::VcoreSetpointSubmit => {
                if self.serial_port.is_some() && !self.core_set.is_empty() {
                    let mut serial_buf: Vec<u8> = vec![0; 0];
                    serial_buf.push(0x00);
                    serial_buf.push(0x21);
                    let data = ulinear16::from(self.core_set.parse::<f32>().unwrap()).to_be_bytes();
                    serial_buf.push(data[1]);
                    serial_buf.push(data[0]);
                    self.send_serial(serial_buf);
                }
            }
            Message::VmemSetpointSubmit => {
                if !self.mem_set.is_empty() {
                    let mut serial_buf: Vec<u8> = vec![0; 0];
                    serial_buf.push(0x01);
                    serial_buf.push(0x21);
                    let data = ulinear16::from(self.mem_set.parse::<f32>().unwrap()).to_be_bytes();
                    serial_buf.push(data[1]);
                    serial_buf.push(data[0]);
                    self.send_serial(serial_buf);
                }
            }
            Message::VcoreCurrentSubmit => {
                if !self.core_lim.is_empty() {
                    let mut serial_buf: Vec<u8> = vec![0; 0];
                    serial_buf.push(0x00);
                    serial_buf.push(0x46);
                    let data = slinear11::from(self.core_lim.parse::<f32>().unwrap()).to_be_bytes();
                    serial_buf.push(data[1]);
                    serial_buf.push(data[0]);
                    self.send_serial(serial_buf);
                }
            }
            Message::VmemCurrentSubmit => {
                if !self.mem_lim.is_empty() {
                    let mut serial_buf: Vec<u8> = vec![0; 0];
                    serial_buf.push(0x01);
                    serial_buf.push(0x46);
                    let data = slinear11::from(self.mem_lim.parse::<f32>().unwrap()).to_be_bytes();
                    serial_buf.push(data[1]);
                    serial_buf.push(data[0]);
                    self.send_serial(serial_buf);
                }
            }
        }
        Task::none()
    }

    fn send_serial(&mut self, mut command: Vec<u8>) {
        match &mut self.serial_port {
            Some(val) => match val.write(command.as_mut_slice()) {
                Ok(_val) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_val) => {}
            },
            None => {}
        }
    }

    fn stop_data(&mut self) {
        println!("Stopping Data Collection");
        self.data_collect_time = -1;
        self.data_collect_file = None;
    }
}
#[derive(Default)]
struct DataChart {
    data_points: VecDeque<(DateTime<Utc>, f32)>,
    theme: Theme,
}

impl DataChart {
    fn default() -> Self {
        Self {
            data_points: VecDeque::new(),
            theme: DARK_THEME,
        }
    }

    fn view(&self) -> Element<Message> {
        let chart = ChartWidget::new(self).width(Length::Fill);
        chart.into()
    }
}

impl Chart<Message> for DataChart {
    type State = ();
    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, mut builder: ChartBuilder<DB>) {
        use plotters::prelude::*;

        let text_color: [u8; 4] = self.theme.palette().text.into_rgba8();
        let text_color: RGBColor = RGBColor(text_color[0], text_color[1], text_color[2]);

        let primary_color: [u8; 4] = self.theme.palette().primary.into_rgba8();
        let primary_color: RGBColor =
            RGBColor(primary_color[0], primary_color[1], primary_color[2]);

        let font_style = ("Noto Sans", 15)
            .into_font()
            .transform(FontTransform::Rotate90)
            .color(&text_color.mix(0.65));

        // Generate Timescale
        let newest_time = self
            .data_points
            .front()
            .unwrap_or(&(DateTime::from_timestamp(0, 0).unwrap(), 0.))
            .0;
        let oldest_time = newest_time - chrono::Duration::seconds(60);

        // Build the Graph
        let mut chart = builder
            .x_label_area_size(48)
            .y_label_area_size(48)
            .margin(20)
            .margin_left(30)
            .margin_right(48)
            .caption(
                "Memory Channel Voltage",
                ("Noto Sans", 20).with_color(&text_color.mix(0.65)),
            )
            .build_cartesian_2d(oldest_time..newest_time, -0.1_f32..3.0_f32)
            .expect("failed to build chart");

        // Build Legend and Text
        chart
            .configure_mesh()
            .bold_line_style(text_color.mix(0.1))
            .bold_line_style(text_color.mix(0.1))
            .light_line_style(text_color.mix(0.05))
            .axis_style(ShapeStyle::from(text_color.mix(0.45)).stroke_width(2))
            .y_labels(10)
            .y_desc("mem (V)")
            .x_desc("Time (s)")
            .y_label_style(font_style.clone())
            .y_label_formatter(&|y| format!("{}", y))
            .x_label_style(font_style.clone())
            .x_label_formatter(&|x| x.format("%M:%S").to_string())
            .draw()
            .expect("failed to draw chart mesh");

        // Truncate OOB Data
        let mut data: VecDeque<(DateTime<Utc>, f32)> = Vec::new().into();
        for point in self.data_points.clone() {
            if point.0 > oldest_time {
                data.push_front(point);
            }
        }

        // Draw the Line
        chart
            .draw_series(LineSeries::new(data, primary_color))
            .expect("failed to draw area series");
    }
}

struct App {
    device: Device,
    core_set: String,
    core_lim: String,
    mem_set: String,
    mem_lim: String,
    data_collect_time: i32,
    data_collect_file: Option<File>,
    chart: DataChart,
    theme: Theme,
    selected_port: Option<String>,
    serial_port: Option<Box<dyn serialport::SerialPort>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            device: Device::default(),
            core_set: String::default(),
            core_lim: String::default(),
            mem_set: String::default(),
            mem_lim: String::default(),
            data_collect_time: -1,
            data_collect_file: None,
            chart: DataChart::default(),
            theme: DARK_THEME,
            selected_port: None,
            serial_port: None,
        }
    }
}

#[derive(Default, bincode::Decode, bincode::Encode)]
pub struct Device {
    core: Channel,
    mem: Channel,
}

#[derive(Default, bincode::Decode, bincode::Encode)]
struct Channel {
    voltage: f32,
    voltage_setpoint: f32,
    current: f32,
    current_limit: f32,
    temperature: f32,
}

#[derive(Debug, Clone)]
enum Message {
    ThemeSwitch(bool),
    Update(Instant),
    StartCollectData(Option<rfd::FileHandle>),
    CollectData,
    // Vcore Updates
    VcoreVoltageUpdate(String),
    VcoreCurrentUpdate(String),
    VcoreSetpointSubmit,
    VcoreCurrentSubmit,
    // Vmem Updates
    VmemVoltageUpdate(String),
    VmemCurrentUpdate(String),
    VmemSetpointSubmit,
    VmemCurrentSubmit,
    // Serial Stuff
    PortSelected(String),
    PortOpen,
}

fn theme(state: &App) -> Theme {
    state.theme.clone()
}
