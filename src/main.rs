use promptuity::{themes::FancyTheme, Promptuity, Term};
use tokio::main;
use utils::{menus::query_menu, tracker::EpisodeTracker};

mod utils;

#[main]
async fn main() {
    let mut terminal = Term::default();
    let mut theme = FancyTheme::default();
    let mut promptuity = Promptuity::new(&mut terminal, &mut theme);
    let mut tracker = EpisodeTracker::new()
        .inspect_err(|err| { eprintln!("No se pudo inciar el tracker: {err}"); });

    let program = query_menu(
        &mut promptuity,
        true,
        &mut tracker.as_mut().ok()
    )
        .await;

    if let Err(err) = program {
        if promptuity.with_outro(format!("Error: {err}")).finish().is_err() {
            eprintln!("Error: {err}");
        }
    }
}
