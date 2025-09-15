use crate::err::blp_err::BlpErr;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Cause {
    Blp(BlpErr),
    Std(Arc<dyn Error + Send + Sync>),
}
