use crate::MainPages::*;

#[derive(Clone)]
pub enum PageToRouteTo {
    Home(HomePage::view::HomePageDrawer),
}
