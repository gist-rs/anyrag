#[cfg(feature = "firebase")]
pub mod firebase;
#[cfg(feature = "firebase")]
pub mod firebase_types;

#[cfg(feature = "github")]
pub mod github;
#[cfg(feature = "github")]
pub mod github_types;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "rss")]
pub mod rss;

#[cfg(feature = "sheets")]
pub mod sheet;

#[cfg(feature = "text")]
pub mod text;

#[cfg(feature = "web")]
pub mod web;
