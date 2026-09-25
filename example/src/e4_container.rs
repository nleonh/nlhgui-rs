// This is free and unencumbered software released into the public domain.
// See example/UNLICENSE.

use nlhgui::{LaunchConfig, colors::{GREEN, RED}, widgets::{Container, Rect}};

fn make_rect() -> Box<Rect> {
    Box::new(Rect::new(60., 60., GREEN))
}

fn make_little_rect() -> Box<Rect> {
    Box::new(Rect::new(40., 40., RED))
}

pub fn run_containers() {
    let mut main_container = Container::new([]).with_spacing(30.0);

    let horiz_container =
        Container::new([make_rect(), make_little_rect(), make_rect()])
            .horizontal()
            .with_spacing(10.0);
    
    main_container.add(horiz_container);

    // Vertical by default
    let vert_container =
        Container::new([make_little_rect(), make_rect(), make_rect()])
            .with_spacing(10.0);
    
    main_container.add(vert_container);

    LaunchConfig::default().with_title(file!()).launch(main_container);
}
