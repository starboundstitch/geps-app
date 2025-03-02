use iced::widget::text_input::cursor::State;
use iced::widget::{
    button, column, container, horizontal_space, rich_text, row, span, text, text_input, Container,
};
use iced::{font, Element, Fill, Length, Task, Theme};

use plotters_iced::{Chart, ChartBuilder, ChartWidget, DrawingBackend};
use rand::Rng;
use std::collections::VecDeque;

use chrono::{DateTime, Utc};

const LIGHT_THEME: Theme = Theme::CatppuccinLatte;
const DARK_THEME: Theme = Theme::CatppuccinFrappe;

pub fn main() -> iced::Result {
    let app = App::default();
    iced::application("Amogus", App::update, App::view)
        .theme(theme)
        .run()
}

impl App {
    fn view(&self) -> Container<Message> {
        container(
            column![
                row![
                    rich_text![span("GEPS-App")
                        .font(font::Font::with_name("Noto Sans"))
                        .size(24)],
                    horizontal_space(),
                    button("").on_press(Message::ThemeSwitch)
                ],
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
                        text_input("Amogus...", &self.vcore.voltage_set)
                            .on_input(Message::VcoreVoltageUpdate)
                            .on_submit(Message::VcoreSetpointSubmit),
                        text_input("Amogus...", &self.vcore.current_lim)
                            .on_input(Message::VcoreCurrentUpdate)
                            .on_submit(Message::VcoreSetpointSubmit),
                    ]
                    .spacing(10)
                    .padding(10),
                    column![
                        text("VMEM"),
                        text_input("Amogus...", &self.vmem.voltage_set)
                            .on_input(Message::VmemVoltageUpdate)
                            .on_submit(Message::VmemSetpointSubmit),
                        text_input("Amogus...", &self.vmem.current_lim)
                            .on_input(Message::VmemCurrentUpdate)
                            .on_submit(Message::VmemSetpointSubmit)
                    ]
                    .spacing(10)
                    .padding(10),
                ],
                self.chart.view(),
                button("+").on_press(Message::CounterIncrement),
                text(self.counter),
                button("-").on_press(Message::CounterDecrement),
            ]
            .spacing(10),
        )
        .padding(10)
        .center_x(Fill)
        .into()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ThemeSwitch => {
                if self.theme == LIGHT_THEME {
                    self.theme = DARK_THEME;
                } else {
                    self.theme = LIGHT_THEME;
                }
                self.chart.theme = self.theme.clone();
            }
            Message::CounterIncrement => {
                self.counter += 1;
                self.chart
                    .data_points
                    .push_front((Utc::now(), rand::rng().random_range(-10..10) as f32));
                // let input = iced::Task::run(text_input::select_all(text_input::Id::new("rawr")));
                // input.unfocus();
                // text_input::State::unfocus(input.unfocus());
                // text_input::Id::new("rawr").type_id;
                // return text_input::focus();
            }
            Message::CounterDecrement => {
                self.counter -= 1;
            }
            Message::VcoreVoltageUpdate(val) => {
                self.vcore.voltage_set = val.clone();
            }
            Message::VcoreCurrentUpdate(val) => {
                self.vcore.current_lim = val.clone();
            }
            Message::VmemVoltageUpdate(val) => {
                self.vmem.voltage_set = val.clone();
            }
            Message::VmemCurrentUpdate(val) => {
                self.vmem.current_lim = val.clone();
            }
            Message::VcoreSetpointSubmit => {
            }
            Message::VmemSetpointSubmit => {
            }
        }
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
        println!("Data Points: {:?}", self.data_points);
        let chart = ChartWidget::new(self).width(Length::Fill);
        chart.into()
    }
}

impl Chart<Message> for DataChart {
    type State = ();
    fn build_chart<DB: DrawingBackend>(&self, state: &Self::State, mut builder: ChartBuilder<DB>) {
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
            .x_label_area_size(28)
            .y_label_area_size(28)
            .margin(20)
            .margin_right(48)
            .build_cartesian_2d(oldest_time..newest_time, -10.0_f32..10.0_f32)
            .expect("failed to build chart");

        // Build Legend and Text
        chart
            .configure_mesh()
            .bold_line_style(text_color.mix(0.1))
            .bold_line_style(text_color.mix(0.1))
            .light_line_style(text_color.mix(0.05))
            .axis_style(ShapeStyle::from(text_color.mix(0.45)).stroke_width(2))
            .y_labels(10)
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
    counter: i64,
    vcore: Channel,
    vmem: Channel,
    chart: DataChart,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            counter: 0,
            vcore: Channel::default(),
            vmem: Channel::default(),
            chart: DataChart::default(),
            theme: DARK_THEME,
        }
    }
}

#[derive(Default)]
struct Channel {
    voltage_set: String,
    current_lim: String,
}

#[derive(Debug, Clone)]
enum Message {
    ThemeSwitch,
    CounterIncrement,
    CounterDecrement,
    // Vcore Updates
    VcoreVoltageUpdate(String),
    VcoreCurrentUpdate(String),
    VcoreSetpointSubmit,
    // Vmem Updates
    VmemVoltageUpdate(String),
    VmemCurrentUpdate(String),
    VmemSetpointSubmit,
}

fn theme(state: &App) -> Theme {
    state.theme.clone()
}
