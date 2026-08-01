use iced::widget::{column, container, row, text};
use iced::{Element, Length, Subscription, Task};
use ruviz::prelude::Plot;
use ruviz_iced::{Message as PlotMessage, Plot3DState, PlotState, plot, plot3d};

struct Dashboard {
    two_d: PlotState,
    three_d: Plot3DState,
}

enum Message {
    TwoD(PlotMessage),
    ThreeD(PlotMessage),
}

impl Dashboard {
    fn new() -> (Self, Task<Message>) {
        let mut two_d = PlotState::static_view(
            Plot::new()
                .bar(&[1.0, 2.0, 3.0], &[3.0, 7.0, 5.0])
                .title("Static 2D"),
        )
        .fill();
        let mut three_d = Plot3DState::static_view(
            ruviz::line3d(
                &[-1.0, -0.5, 0.0, 0.5, 1.0],
                &[0.0, 0.7, 0.0, -0.7, 0.0],
                &[-0.5, 0.0, 0.5, 0.0, -0.5],
            )
            .title("Static 3D"),
        )
        .expect("example 3D data is valid")
        .fill();
        let tasks = Task::batch([
            two_d.request_render().into_task().map(Message::TwoD),
            three_d.request_render().into_task().map(Message::ThreeD),
        ]);
        (Self { two_d, three_d }, tasks)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TwoD(message) => self.two_d.update(message).into_task().map(Message::TwoD),
            Message::ThreeD(message) => self
                .three_d
                .update(message)
                .into_task()
                .map(Message::ThreeD),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.two_d.subscription().map(Message::TwoD)
    }

    fn view(&self) -> Element<'_, Message> {
        let two_d: Element<'_, PlotMessage> = plot(&self.two_d).into();
        let three_d: Element<'_, PlotMessage> = plot3d(&self.three_d).into();
        column![
            text("Static ruviz images embedded in Iced"),
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
        ]
        .padding(12)
        .spacing(8)
        .into()
    }
}

fn main() -> iced::Result {
    iced::application(Dashboard::new, Dashboard::update, Dashboard::view)
        .subscription(Dashboard::subscription)
        .run()
}
