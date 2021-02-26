use std::error::Error;

use async_trait::async_trait;
use url::Url;

use crate::Calendar;

#[async_trait]
pub trait CalDavSource {
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars(&self) -> Result<&Vec<Calendar>, Box<dyn Error>>;
    /// Returns the current calendars that this source contains
    /// This function may trigger an update (that can be a long process, or that can even fail, e.g. in case of a remote server)
    async fn get_calendars_mut(&mut self) -> Result<Vec<&mut Calendar>, Box<dyn Error>>;

    //
    //
    // TODO: find a better search key (do calendars have a unique ID?)
    // TODO: search key should be a reference
    //
    /// Returns the calendar matching the URL
    async fn get_calendar(&self, url: Url) -> Option<&Calendar>;
    /// Returns the calendar matching the URL
    async fn get_calendar_mut(&mut self, url: Url) -> Option<&mut Calendar>;

}
