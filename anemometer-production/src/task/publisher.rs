use crate::global_settings;
use crate::state::*;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use log::*;

pub async fn wind_speed_task() {
    let publisher = APPLICATION_DATA_CHANNEL.publisher().unwrap();
    let mut app_event = APPLICATION_EVENT_CHANNEL.subscriber().unwrap();
    info!("Publisher Task Started");
    loop {
        let (timer_fired, app_state_change) = match select(
            Timer::after(Duration::from_secs(
                global_settings::DATA_REPORTING_INTERVAL,
            )),
            app_event.next_message_pure(),
        )
        .await
        {
            Either::First(_) => (Some(true), None),
            Either::Second(app_state_change) => (None, Some(app_state_change)),
        };
        if let Some(ApplicationStateChange::OTAUpdateStarted) = app_state_change {
            info!(
                "wind_speed_task OTA Update started shutting down wind_speed_demo_publisher task"
            );
            break;
        }

        if let Some(send_needed) = timer_fired {
            if send_needed {
                publisher
                    .publish(ApplicationDataChange::ReportWindData)
                    .await;
            }
        }
    }
}
