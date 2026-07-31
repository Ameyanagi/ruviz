use iced::widget::{column, container, row, text};
use iced::{Element, Length, Subscription, Task};
use ruviz::prelude::Plot;
use ruviz_iced::{Message as PlotMessage, Plot3DState, PlotState, plot, plot3d};

struct Dashboard {
    two_d: PlotState,
    three_d: Plot3DState,
    status: String,
}

enum Message {
    TwoD(PlotMessage),
    ThreeD(PlotMessage),
}

impl Dashboard {
    fn new() -> (Self, Task<Message>) {
        let mut two_d = PlotState::interactive(
            Plot::new()
                .line(&[0.0, 1.0, 2.0, 3.0, 4.0], &[0.0, 1.0, 4.0, 9.0, 16.0])
                .title("2D"),
        )
        .fill();
        let mut three_d = Plot3DState::interactive(
            ruviz::scatter3d(
                &[-1.0, -0.4, 0.2, 0.8, 1.0],
                &[0.0, 0.8, -0.7, 0.5, -0.2],
                &[0.4, -0.6, 0.9, -0.3, 0.7],
            )
            .title("3D"),
        )
        .expect("example 3D data is valid")
        .fill();
        let tasks = Task::batch([
            two_d.request_render().into_task().map(Message::TwoD),
            three_d.request_render().into_task().map(Message::ThreeD),
        ]);
        (
            Self {
                two_d,
                three_d,
                status: "2D: pan/zoom/select · 3D: orbit/pan/zoom/pick · right-click either plot"
                    .to_owned(),
            },
            tasks,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let (task, event, route) = match message {
            Message::TwoD(message) => {
                let (task, event) = self.two_d.update(message).into_parts();
                (task, event, Route::TwoD)
            }
            Message::ThreeD(message) => {
                let (task, event) = self.three_d.update(message).into_parts();
                (task, event, Route::ThreeD)
            }
        };
        if let Some(event) = event.last() {
            self.status = format!("{event:?}");
        }
        match route {
            Route::TwoD => task.map(Message::TwoD),
            Route::ThreeD => task.map(Message::ThreeD),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.two_d.subscription().map(Message::TwoD),
            self.three_d.subscription().map(Message::ThreeD),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let two_d: Element<'_, PlotMessage> = plot(&self.two_d).into();
        let three_d: Element<'_, PlotMessage> = plot3d(&self.three_d).into();
        column![
            text("ruviz native Iced dashboard"),
            row![
                container(two_d.map(Message::TwoD))
                    .width(Length::Fill)
                    .height(Length::Fill),
                container(three_d.map(Message::ThreeD))
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .spacing(8)
            .height(Length::Fill),
            text(&self.status),
        ]
        .padding(12)
        .spacing(8)
        .into()
    }
}

enum Route {
    TwoD,
    ThreeD,
}

fn main() -> iced::Result {
    iced::application(Dashboard::new, Dashboard::update, Dashboard::view)
        .subscription(Dashboard::subscription)
        .run()
}
