use base64::Engine;

use super::{PaginatedResult, Pagination};

pub fn decode_offset(token: &Option<String>) -> Option<u64> {
    token.as_ref().and_then(|t| {
        let decoded = base64::engine::general_purpose::STANDARD.decode(t).ok()?;
        String::from_utf8(decoded).ok()?.parse().ok()
    })
}

pub fn encode_offset(offset: u64) -> String {
    base64::engine::general_purpose::STANDARD.encode(offset.to_string())
}

pub fn calculate_next_token<T>(items: Vec<T>, page_size: u32, offset: u64) -> PaginatedResult<T> {
    let has_more = items.len() > page_size as usize;
    let items = items.into_iter().take(page_size as usize).collect();

    let next_page_token = if has_more {
        Some(encode_offset(offset + page_size as u64))
    } else {
        None
    };

    PaginatedResult { items, next_page_token }
}

pub fn pagination_offset(pagination: &Pagination) -> u64 {
    decode_offset(&pagination.page_token).unwrap_or(0)
}
