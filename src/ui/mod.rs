mod app;
mod event_app;
mod events;
pub mod gui; // Make the gui module public
mod history;
mod models;
mod widgets;

pub use event_app::EventBasedApp;
pub use app::App;
 // Make all public items in gui module available
