//! High-performance PDE method-of-lines models across reaction-diffusion, hyperbolic, wave, and curvilinear classes.

pub mod allen_cahn;
pub mod burgers;
pub mod gray_scott;
pub mod sine_gordon;
pub mod spherical;

pub use allen_cahn::AllenCahn2D;
pub use burgers::Burgers2D;
pub use gray_scott::GrayScott2D;
pub use sine_gordon::SineGordon2D;
pub use spherical::SphericalDiffusion2D;
