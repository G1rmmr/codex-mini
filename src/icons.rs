//! Small action icons drawn as vectors; no icon font or image loader.
use iced::{
    Color, Point, Rectangle, Renderer, Theme, mouse,
    widget::canvas::{self, Frame, Geometry, Path, Stroke},
};

#[derive(Clone, Copy)]
pub enum Kind {
    Sidebar,
    NewChat,
    More,
    Chevron,
    Down,
    Settings,
    Send,
    Stop,
    Copy,
    Source,
    Sliders,
    Back,
    Terminal,
}
pub struct Icon {
    pub kind: Kind,
    pub color: Color,
}
impl<Message> canvas::Program<Message> for Icon {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.scale(bounds.width / 24.0);
        let stroke = Stroke::default()
            .with_color(self.color)
            .with_width(1.6)
            .with_line_cap(canvas::LineCap::Round)
            .with_line_join(canvas::LineJoin::Round);
        let segments: &[&[(f32, f32)]] = match self.kind {
            Kind::Sidebar => &[
                &[(4., 4.), (20., 4.), (20., 20.), (4., 20.), (4., 4.)],
                &[(9., 4.), (9., 20.)],
            ],
            Kind::NewChat => &[
                &[(10., 5.), (5., 5.), (5., 19.), (19., 19.), (19., 14.)],
                &[
                    (12., 12.),
                    (13., 8.),
                    (19., 2.),
                    (22., 5.),
                    (16., 11.),
                    (12., 12.),
                ],
            ],
            Kind::Chevron => &[&[(9., 5.), (16., 12.), (9., 19.)]],
            Kind::Down => &[&[(6., 9.), (12., 15.), (18., 9.)]],
            Kind::Send => &[
                &[(12., 19.), (12., 5.)],
                &[(6., 11.), (12., 5.), (18., 11.)],
            ],
            Kind::Copy => &[
                &[(9., 4.), (20., 4.), (20., 15.), (9., 15.), (9., 4.)],
                &[(5., 9.), (4., 9.), (4., 20.), (15., 20.), (15., 19.)],
            ],
            Kind::Source => &[
                &[(8., 6.), (2., 12.), (8., 18.)],
                &[(16., 6.), (22., 12.), (16., 18.)],
            ],
            Kind::Sliders => &[
                &[(4., 7.), (8., 7.)],
                &[(12., 7.), (20., 7.)],
                &[(4., 17.), (13., 17.)],
                &[(17., 17.), (20., 17.)],
                &[(8., 4.), (12., 4.), (12., 10.), (8., 10.), (8., 4.)],
                &[(13., 14.), (17., 14.), (17., 20.), (13., 20.), (13., 14.)],
            ],
            Kind::Back => &[&[(14., 5.), (7., 12.), (14., 19.)]],
            Kind::Settings => &[],
            Kind::Terminal => &[
                &[(5., 6.), (11., 12.), (5., 18.)],
                &[(13., 18.), (20., 18.)],
            ],
            Kind::More | Kind::Stop => &[],
        };
        for segment in segments {
            let path = Path::new(|p| {
                p.move_to(Point::new(segment[0].0, segment[0].1));
                for &(x, y) in &segment[1..] {
                    p.line_to(Point::new(x, y));
                }
            });
            frame.stroke(&path, stroke);
        }
        match self.kind {
            Kind::More => {
                for x in [5., 12., 19.] {
                    frame.fill(&Path::circle(Point::new(x, 12.), 1.5), self.color);
                }
            }
            Kind::Stop => {
                frame.fill_rectangle(Point::new(7., 7.), iced::Size::new(10., 10.), self.color)
            }
            Kind::Settings => {
                frame.stroke(&Path::circle(Point::new(12., 12.), 6.), stroke);
                frame.stroke(&Path::circle(Point::new(12., 12.), 2.3), stroke);
                for i in 0..8 {
                    let a = i as f32 * std::f32::consts::FRAC_PI_4;
                    frame.stroke(
                        &Path::line(
                            Point::new(12. + 6. * a.cos(), 12. + 6. * a.sin()),
                            Point::new(12. + 8. * a.cos(), 12. + 8. * a.sin()),
                        ),
                        stroke,
                    );
                }
            }
            _ => {}
        }
        vec![frame.into_geometry()]
    }
}
pub fn view<'a, Message: 'a>(kind: Kind, color: Color, size: f32) -> iced::Element<'a, Message> {
    iced::widget::canvas(Icon { kind, color })
        .width(size)
        .height(size)
        .into()
}
