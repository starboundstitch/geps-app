use iced::widget::{button, column, container, row, slider, text, text_input, Column, Container};
use iced::{font, Element, Fill, Length, Task, Theme};

use plotters::prelude::*;
use plotters_iced::{Chart, ChartBuilder, ChartWidget, DrawingBackend};
use rand::Rng;
use std::{collections::VecDeque, time::Instant};

use chrono::{DateTime, Utc};

const PLOT_LINE_COLOR: RGBColor = RGBColor(0, 175, 255);
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
                button("+").on_press(Message::CounterIncrement),
                text(self.counter),
                button("-").on_press(Message::CounterDecrement),
                text(self.slider),
                slider(0.8..=2.0, self.slider, Message::SliderUpdate),
                row![
                    text("Voltage Setpoint:"),
                    text_input("Retards...", &self.voltage_set)
                    text_input("Amogus...", &self.voltage_set)
                        .id(text_input::Id::new("rawr"))
                        .on_input(Message::VoltageSetpointUpdate)
                        .on_submit(Message::VoltageSetpointSubmit)
                ]
                .spacing(10)
                .padding(10),
                self.chart.view()
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
                self.chart.data_points.push_front((
                    // rand::rng().random_range(1..100) as f32,
                    Utc::now(),
                    rand::rng().random_range(-10..10) as f32,
                ));
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
        let newest_time = self
            .data_points
            .front()
            .unwrap_or(&(DateTime::from_timestamp(0, 0).unwrap(), 0.))
            .0;
        let oldest_time = newest_time - chrono::Duration::seconds(60);

        // Truncate OOB Data
        let mut data: VecDeque<(DateTime<Utc>, f32)> = Vec::new().into();
        for point in self.data_points.clone() {
            if point.0 > oldest_time {
                data.push_front(point);
            }
        }

        let mut chart = builder
            .x_label_area_size(28)
            .y_label_area_size(28)
            .margin(20)
            .build_cartesian_2d(oldest_time..newest_time, -10.0_f32..10.0_f32)
            .expect("failed to build chart");

        chart
            .configure_mesh()
            .bold_line_style(plotters::style::colors::BLUE.mix(0.1))
            .light_line_style(plotters::style::colors::BLUE.mix(0.05))
            .axis_style(ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1))
            .y_labels(10)
            .y_label_style(
                ("Noto Sans", 15)
                    .into_font()
                    .color(&plotters::style::colors::BLUE.mix(0.65))
                    .transform(FontTransform::Rotate90),
            )
            .y_label_formatter(&|y| format!("{}", y))
            .x_label_formatter(&|x| x.format("%M:%S").to_string())
            .draw()
            .expect("failed to draw chart mesh");

        // println!("Displaying Chart");

        chart
            .draw_series(LineSeries::new(
                data, BLACK, // PLOT_LINE_COLOR.mix(0.175),
            ))
            .expect("failed to draw area series");
        // chart
        // .draw_series(LineSeries::new(
        //     self.data_points.iter().cloned(),
        //     PLOT_LINE_COLOR.mix(0.175),
        // ))
        // .expect("failed to draw the line series");
        // chart
        //     .draw_series(LineSeries::new(
        //         (0..100).map(|x| (x as f32, (100 - x) as f32)),
        //         &BLACK,
        //     ))
        //     .expect("you're a failure");
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
