use std::env::args;

use ansi_term::Color::{Red, Cyan};
use promptuity::{prompts::{Input, Select, SelectOption}, themes::FancyTheme, Error, Promptuity, Term};
use tokio::main;
use utils::{mpv::{check_mpv, run_mpv}, scraper::{get_play_links, query_anime, select_chapters}};

mod utils;

macro_rules! promptuity_error {
    ($p:expr, $s:expr) => {{
        $p.with_outro(Red.paint($s)).finish()?;
        return Err(Error::Cancel);
    }};
}

async fn play_content(verbose: bool) -> Result<(), Error> {
    let mut term = Term::default();
    let mut theme = FancyTheme::default();
    let mut promptuity = Promptuity::new(&mut term, &mut theme);

    promptuity.term().clear()?;
    promptuity.with_intro("Bienvenido a quanires.").begin()?;

    if !check_mpv() {
        promptuity_error!(
            promptuity,
            "MPV no se encuentra en la PATH, añadelo o instalalo: https://mpv.io/installation/"
        );
    }

    let anime_list = match query_anime(
        &promptuity.prompt(
            &mut Input::new("Que deseas ver?"))?
    )
        .await
    {
        Err(_) => promptuity_error!(promptuity, "Hubo un error al obtener los resultados."),
        Ok(res) => res
    };

    if anime_list.is_empty() {
        promptuity_error!(promptuity, "La lista de resultados esta vacia.");
    }

    let anime_list_c = anime_list.clone();

    let selected_anime = promptuity.prompt(&mut Select::new(
        "Selecciona un anime.",
        anime_list_c
            .iter()
            .map(|anime| SelectOption::new(anime.name(), anime.url()))
            .collect()
    ))?;

    let selected_anime = anime_list
        .into_iter()
        .find(|entry| entry.url() == selected_anime)
        .unwrap(); // it does always exist.

    let chapters = match select_chapters(&selected_anime.url()).await {
        Err(_) => promptuity_error!(promptuity, "Hubo un error al obtener los capitulos."),
        Ok(result) => result
    };

    if chapters.is_empty() {
        promptuity_error!(promptuity, "Este episodio no tiene capitulos.");
    }

    let chapter_number = promptuity.prompt(
        &mut Select::new(
            "Selecciona un episodio.",
            chapters
                .iter()
                .map(|chapter| SelectOption::new(
                    format!("Capitulo {}", chapter.number()), chapter.number())
                )
                .collect()
        )
    )?;

    let chapter_links = match get_play_links(
        chapters.iter()
            .find(|c| c.number() == chapter_number)
            .unwrap()
            .url()
    )
        .await
    {
        Err(_) => {
            promptuity_error!(promptuity, "Hubo un error al obtener los enlaces.")
        },
        Ok(result) => result
    };

    promptuity.step(Cyan.paint(format!("Cargando {}...", selected_anime.name())))?;
    let mut could_load = false;

    let selected_anime = selected_anime.clone();

    for link in chapter_links {
        match run_mpv(selected_anime.name(), link.clone()).await {
            Ok(res) if res => {
                promptuity.term().clear()?;

                promptuity.with_intro(
                    format!("{} | Capitulo {}", selected_anime.name(), chapter_number)
                ).begin()?;

                if verbose {
                    promptuity.step(format!("VERBOSE: Reproduciendo de {}", link))?;
                }

                promptuity.prompt(
                    &mut Select::new(
                        "Que deseas hacer?",
                        vec![
                            SelectOption::new("Proximo episodio", "next"),
                            SelectOption::new("Anterior episodio", "last"),
                            SelectOption::new("Seleccionar episodio", "select"),
                            SelectOption::new("Ver otro anime", "another"),
                            SelectOption::new("Salir", "exit")
                        ]
                    )
                )?;
                could_load = true;
            },
            _ => {
                promptuity.error(link)?;
                continue;
            },
        }
    }

    if !could_load {
        promptuity_error!(promptuity, "No se pudo cargar.");
    }

    Ok(())
}

#[main]
async fn main() {
    // This is abstracted as a one-time-only function
    // for the sake of handling errors asyncronously.
    let _ = play_content(
        args()
            .collect::<Vec<String>>().contains(&"--verbose".to_string())
    )
    .await;
}
