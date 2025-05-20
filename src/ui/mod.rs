mod app;
mod event_app;
mod events;
pub mod gui; // Make the gui module public
mod history;
mod models;
mod widgets;

pub use app::App;
pub use event_app::EventBasedApp;
// Make all public items in gui module available
