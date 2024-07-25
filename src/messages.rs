pub mod melee_mania {
    use std::time::Duration;

    pub fn info_message(delay: &Duration, duration: &Duration) -> String {
        format!(
            "HELLVENTS | INFO\n
The mini game MELEE MANIA will start in {}. \
For a period of {}, only melee weapons will be allowed.\n
You will receive a message when the mini game has started and ended.\n
Invalid kills result in penalities!
1 & 2 Invalid Kills: Forced Redeploy
3+ Invalid Kills: Kick",
            humantime::format_duration(*delay),
            humantime::format_duration(*duration)
        )
    }

    pub fn start_message(duration: &Duration) -> String {
        format!(
            "HELLVENTS | START\n
The mini game MELEE MANIA has started. \
For a period of {}, only melee weapons will be allowed.\n
You will receive a message when the mini game has ended.\n
Invalid kills result in penalities!
1 & 2 Invalid Kills: Forced Redeploy
3+ Invalid Kills: Kick",
            humantime::format_duration(*duration)
        )
    }

    pub fn running_message(remaining: &Duration) -> String {
        format!(
            "HELLVENTS | RUNNING\n\nThe mini game MELEE MANIA is currently running.\n
For a period of {}, only melee weapons will be allowed.\n
You will receive a message when the mini game has ended.\n
Invalid kills result in penalities!
1 & 2 Invalid Kills: Forced Redeploy
3+ Invalid Kills: Kick",
            humantime::format_duration(*remaining)
        )
    }

    pub fn end_message() -> String {
        format!(
            "HELLVENTS | END\n
The mini game MELEE MANIA has ended.\n
Thanks for participating.\n
-----\n{}",
            open_source_disclaimer()
        )
    }

    pub fn open_source_disclaimer() -> &'static str {
        "Hellvents is open source and freely available for use, from the community for it!\n
GitHub:
https://github.com/Lelleck/hellvents"
    }
}
