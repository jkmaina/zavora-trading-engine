//! Standardized API response formats
//!
//! This module provides a set of consistent response types to be used by all API endpoints.
//! Using these standardized formats ensures a consistent API experience for clients.

use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use utoipa::ToSchema;

/// A standardized API response wrapper for single resource responses
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    /// The response data
    pub data: T,
    /// Optional metadata about the response (e.g. request ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMetadata>,
}

/// Additional metadata about the response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResponseMetadata {
    /// Optional request ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Optional additional metadata fields
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// A standardized API response wrapper for list/collection responses
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiListResponse<T> {
    /// The list of items
    pub data: Vec<T>,
    /// Optional metadata about the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMetadata>,
}

/// A standardized API response wrapper for paginated list responses
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// The list of items in this page
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationMetadata,
    /// Optional additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMetadata>,
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationMetadata {
    /// The current page number (1-based)
    pub page: usize,
    /// The number of items per page
    pub per_page: usize,
    /// The total number of items
    pub total: usize,
    /// The total number of pages
    pub total_pages: usize,
}

// Implementation to convert ApiResponse to axum Response
impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize + Debug,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

// Implementation to convert ApiListResponse to axum Response
impl<T> IntoResponse for ApiListResponse<T>
where
    T: Serialize + Debug,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

// Implementation to convert PaginatedResponse to axum Response
impl<T> IntoResponse for PaginatedResponse<T>
where
    T: Serialize + Debug,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

// Utility methods for creating responses

impl<T> ApiResponse<T> {
    /// Create a new API response with just data
    pub fn new(data: T) -> Self {
        Self {
            data,
            meta: None,
        }
    }

    /// Create a new API response with data and metadata
    pub fn with_metadata(data: T, meta: ResponseMetadata) -> Self {
        Self {
            data,
            meta: Some(meta),
        }
    }

    /// Create a new API response with data and request ID
    pub fn with_request_id(data: T, request_id: String) -> Self {
        Self {
            data,
            meta: Some(ResponseMetadata {
                request_id: Some(request_id),
                extra: None,
            }),
        }
    }
}

impl<T> ApiListResponse<T> {
    /// Create a new list response with just data
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            meta: None,
        }
    }

    /// Create a new list response with data and metadata
    pub fn with_metadata(data: Vec<T>, meta: ResponseMetadata) -> Self {
        Self {
            data,
            meta: Some(meta),
        }
    }

    /// Create a new list response with data and request ID
    pub fn with_request_id(data: Vec<T>, request_id: String) -> Self {
        Self {
            data,
            meta: Some(ResponseMetadata {
                request_id: Some(request_id),
                extra: None,
            }),
        }
    }
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(
        data: Vec<T>,
        page: usize,
        per_page: usize,
        total: usize,
    ) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };

        Self {
            data,
            pagination: PaginationMetadata {
                page,
                per_page,
                total,
                total_pages,
            },
            meta: None,
        }
    }

    /// Create a new paginated response with metadata
    pub fn with_metadata(
        data: Vec<T>,
        page: usize,
        per_page: usize,
        total: usize,
        meta: ResponseMetadata,
    ) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };

        Self {
            data,
            pagination: PaginationMetadata {
                page,
                per_page,
                total,
                total_pages,
            },
            meta: Some(meta),
        }
    }
}