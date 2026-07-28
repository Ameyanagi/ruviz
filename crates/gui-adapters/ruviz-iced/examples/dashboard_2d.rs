use iced::widget::{column, container, text};
use iced::{Element, Length, Subscription, Task};
use ruviz::prelude::Plot;
use ruviz_iced::{Message as PlotMessage, PlotState, plot};

struct Dashboard {
    plot: PlotState,
    status: String,
}

enum Message {
    Plot(PlotMessage),
}

impl Dashboard {
    fn new() -> (Self, Task<Message>) {
        let mut plot = PlotState::interactive(
            Plot::new()
                .line(&[0.0, 1.0, 2.0, 3.0, 4.0], &[0.0, 1.0, 4.0, 9.0, 16.0])
                .title("Interactive Iced plot"),
        )
        .fill();
        let initial = plot.request_render().into_task().map(Message::Plot);
        (
            Self {
                plot,
                status: "Drag to pan, scroll to zoom, click to select".to_owned(),
            },
            initial,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plot(message) => {
                let update = self.plot.update(message);
                if let Some(event) = update.event() {
                    self.status = format!("{event:?}");
                }
                update.into_task().map(Message::Plot)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.plot.subscription().map(Message::Plot)
    }

    fn view(&self) -> Element<'_, Message> {
        let plot: Element<'_, PlotMessage> = plot(&self.plot).into();
        column![
            text("ruviz + Iced"),
            container(plot.map(Message::Plot))
                .width(Length::Fill)
                .height(Length::Fill),
            text(&self.status),
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
