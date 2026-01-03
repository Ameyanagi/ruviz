// Rendering backend interface (future implementation)

pub trait Renderer {
    type Error;

    fn render(&self) -> Result<(), Self::Error>;
}
