use std::{sync::Arc, time::Duration};

use tauri::AppHandle;

use crate::app_state::AppState;

pub fn start(state: Arc<AppState>, app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        state.refresh(app.clone()).await;
        loop {
            let minutes = state.view_state().await.refresh_interval_minutes;
            tokio::time::sleep(Duration::from_secs(u64::from(minutes) * 60)).await;
            state.refresh(app.clone()).await;
        }
    });
}
