use tui::backend;
use tui::widgets::{Block, Borders, Widget};
use tui::Terminal;

use std::io;

pub enum AppEvent<I> {
  Input(I),
  Tick,
}

pub struct App<'a> {
  pub title: &'a str,
  pub should_quit: bool,
}

impl<'a> App<'a> {
  pub fn new(title: &'a str) -> App<'a> {
    App {
      title,
      should_quit: false,
    }
  }

  pub fn on_up(&mut self) {
    // TODO:
  }

  pub fn on_down(&mut self) {
    // TODO:
  }

  pub fn on_right(&mut self) {
    // TODO:
  }

  pub fn on_left(&mut self) {
    // TODO:
  }

  pub fn on_key(&mut self, c: char) {
    match c {
      'q' => {
        self.should_quit = true;
      }
      _ => {}
    }
  }

  pub fn on_tick(&mut self) {
    // TODO: update Transfers and iterate connections here
  }

  pub fn draw<B: backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), io::Error> {
    terminal.draw(|mut f| {
      let size = f.size();
      Block::default()
        .title(self.title)
        .borders(Borders::ALL)
        .render(&mut f, size);
    })
  }
}
