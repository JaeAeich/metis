use serde::Deserialize;
use utoipa::IntoParams;

use crate::repositories::Pagination;

const DEFAULT_PAGE_SIZE: u32 = 50;
const MAX_PAGE_SIZE: u32 = 1000;

#[derive(Debug, Deserialize, IntoParams)]
pub struct PageParams {
    #[param(example = 50)]
    pub page_size: Option<u32>,

    #[param(example = "NTA=")]
    pub page_token: Option<String>,
}

impl PageParams {
    pub fn into_pagination(self) -> Pagination {
        Pagination::new(
            self.page_size.unwrap_or(DEFAULT_PAGE_SIZE).min(MAX_PAGE_SIZE),
            self.page_token,
        )
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct LogPageParams {
    #[param(example = 500)]
    pub page_size: Option<u32>,

    #[param(example = 42)]
    pub after_seq: Option<i64>,
}

impl LogPageParams {
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(500).min(MAX_PAGE_SIZE)
    }
}
