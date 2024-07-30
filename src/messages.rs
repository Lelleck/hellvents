pub mod melee_mania {
    use std::time::Duration;

    /*

    HELLVENTS | INTELLIGENCE

    Attention Soldiers!
    Intelligence informs that we will be facing a munitions shortage in 2m lasting, us and the enemy 5m.
    During this time no bullet may be fired to conserve our reserves.
    Ready your blades!

    */

    pub fn info_message(delay: &Duration, duration: &Duration) -> String {
        format!(
            "HELLVENTS | INFO

The mini game MELEE MANIA will start in {}. \
For a period of {}, only melee weapons will be allowed.

You will receive a message when the mini game has started and ended.

Invalid kills result in penalities!
1 & 2 Invalid Kills: Forced Redeploy
3+ Invalid Kills: Kick",
            humantime::format_duration(*delay),
            humantime::format_duration(*duration)
        )
    }

    pub fn start_message(duration: &Duration) -> String {
        format!(
            "HELLVENTS | START

The mini game MELEE MANIA has started. \
For a period of {}, only melee weapons will be allowed.

You will receive a message when the mini game has ended.

Invalid kills result in penalities!
1 & 2 Invalid Kills: Forced Redeploy
3+ Invalid Kills: Kick",
            humantime::format_duration(*duration)
        )
    }

    pub fn running_message(remaining: &Duration) -> String {
        format!(
            "HELLVENTS | RUNNING
            
The mini game MELEE MANIA is currently running.

For a period of {}, only melee weapons will be allowed.

You will receive a message when the mini game has ended.

Invalid kills result in penalities!
1 & 2 Invalid Kills: Forced Redeploy
3+ Invalid Kills: Kick",
            humantime::format_duration(*remaining)
        )
    }

    pub fn end_message() -> String {
        format!(
            "HELLVENTS | END

The mini game MELEE MANIA has ended.

Thanks for participating.

-----

{}",
            super::open_source_disclaimer()
        )
    }
}

pub fn open_source_disclaimer() -> &'static str {
    "Hellvents is open source and freely available for use, from the community for it!

GitHub:
https://github.com/Lelleck/hellvents"
}
