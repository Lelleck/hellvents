use crate::client::{broadcast_message, WsTransceiver};

use super::Event;

#[derive(Clone)]
pub struct NotifyPlayers {
    transceiver: WsTransceiver,
}

impl Event for NotifyPlayers {
    fn start(&self) {
        _ = tokio::spawn(self.clone().run());
    }

    fn stop(&self) {}

    fn is_stopped(&self) -> bool {
        true
    }
}

impl NotifyPlayers {
    pub fn new(transceiver: WsTransceiver) -> Self {
        Self { transceiver }
    }

    async fn run(mut self) {
        let message =
            "Hello, Community!\n
In a few minutes we will be conducting a 5 minute long test with a mini game.\n
We've been playing around with the concept for a few weeks now and can present a (hopefully) functional prototype. \
We think the concept has potential but still aren't sure how to use it and before moving ahead want to figure out if you, the community, would welcome such a concept.\n
That's why this test today to allow you to experience it live and give us feedback and new ideas. \
If you want to say anything be that feedback, your vision, or anything else regarding the matter you can post messages into chat or discuss the matter on the discord server.\n
Have fun!
- Fachi & The Admin Team
            ".to_string();

        broadcast_message(&mut self.transceiver, message).await;
    }
}
