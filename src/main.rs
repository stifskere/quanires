use promptuity::{themes::FancyTheme, Promptuity, Term};
use tokio::main;
use utils::menus::query_menu;

mod utils;

#[main]
async fn main() {
    let mut terminal = Term::default();
    let mut theme = FancyTheme::default();
    let mut promptuity = Promptuity::new(&mut terminal, &mut theme);

    let program = query_menu(
        &mut promptuity,
        true
    )
        .await;

    if let Err(err) = program {
        eprintln!("{err}");
    }
}
