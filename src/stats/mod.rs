//! Statistical computations for advanced plot types
//!
//! This module provides statistical algorithms used by various plot types:
//! - Kernel density estimation (KDE) for violin and density plots
//! - Regression analysis for regplot and residplot
//! - Quantile calculations for boxen plots
//! - Contour extraction using marching squares
//! - Beeswarm algorithm for non-overlapping point placement
//! - Hierarchical clustering for clustermaps

pub mod beeswarm;
pub mod clustering;
pub mod contour;
pub mod kde;
pub mod quantile;
pub mod regression;

pub use beeswarm::beeswarm_positions;
pub use clustering::{Linkage, LinkageMethod, linkage};
pub use contour::{ContourLevel, contour_lines, marching_squares};
pub use kde::{KdeResult, gaussian_kde, kde_1d, kde_2d};
pub use quantile::{letter_values, quantiles};
pub use regression::{RegressionResult, linear_regression, polynomial_regression};
