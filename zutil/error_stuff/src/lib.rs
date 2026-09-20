use protocol::error::DbError;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug)]
pub enum ErrorDetail {
    Db(DbError),
    Col(String),
}

pub trait IntoErrorDetail {
    fn into_detail(self) -> ErrorDetail;
}

impl IntoErrorDetail for DbError {
    fn into_detail(self) -> ErrorDetail {
        ErrorDetail::Db(self)
    }
}

impl IntoErrorDetail for String {
    fn into_detail(self) -> ErrorDetail {
        ErrorDetail::Col(self)
    }
}

impl IntoErrorDetail for &str {
    fn into_detail(self) -> ErrorDetail {
        ErrorDetail::Col(self.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct AppError {
    pub detail: ErrorDetail,
    pub screen: String,
    pub location: String,
}

static APP_DISABLED: AtomicBool = AtomicBool::new(false);
static APP_ERROR: Mutex<Option<AppError>> = Mutex::new(None);

pub fn is_disabled() -> bool {
    APP_DISABLED.load(Ordering::Relaxed)
}

pub fn disable() {
    APP_DISABLED.store(true, Ordering::Relaxed);
}

pub fn enable() {
    APP_DISABLED.store(false, Ordering::Relaxed);
    *APP_ERROR.lock().unwrap() = None;
}

pub fn report_error(error: AppError) {
    *APP_ERROR.lock().unwrap() = Some(error);
    disable();
}

pub fn current_error() -> Option<AppError> {
    APP_ERROR.lock().unwrap().clone()
}

#[macro_export]
macro_rules! unwrap_or_bail {
    ($result:expr, $screen:expr, $location:expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                $crate::report_error($crate::AppError {
                    detail: $crate::IntoErrorDetail::into_detail(e),
                    screen: $screen.to_string(),
                    location: $location.to_string(),
                });
                return;
            }
        }
    };
}
