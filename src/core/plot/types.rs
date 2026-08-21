use super::*;

/// Main Plot struct - the core API entry point for creating visualizations
///
/// `Plot` provides a fluent builder interface for creating plots with multiple
/// data series, styling options, and export capabilities.
///
/// # Architecture
///
/// The Plot struct delegates to focused component managers:
/// - [`PlotConfiguration`] - Title, labels, dimensions, theme, DPI
/// - [`SeriesManager`] - Data series storage, auto-color assignment
/// - [`LayoutManager`] - Legend, grid, ticks, margins, axis limits/scales
/// - [`RenderPipeline`] - Backend selection, parallel/pooled rendering
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
///
/// Plot::new()
///     .line(&x, &y)
///     .color(Color::BLUE)
///     .line_width(2.0)
///     .title("My Plot")
///     .xlabel("X")
///     .ylabel("Y")
///     .save("plot.png")?;
/// ```
///
/// # Builder Pattern
///
/// Series methods (`.line()`, `.scatter()`, `.bar()`) return a [`PlotBuilder<C>`]
/// that auto-finalizes when terminal methods (`.save()`, `.render()`) are called.
/// No explicit `.end_series()` is needed for fluent chaining, including transitions
/// into compatible series builders and styled annotation methods.
#[derive(Clone, Debug)]
pub struct Plot {
    /// Display configuration (title, labels, dimensions, theme)
    pub(super) display: PlotConfiguration,
    /// Series manager (handles all data series and auto-coloring)
    pub(super) series_mgr: SeriesManager,
    /// Layout manager (handles legend, grid, ticks, margins, axis limits)
    pub(super) layout: LayoutManager,
    /// Render pipeline (handles backend selection, parallel/pooled rendering)
    pub(super) render: RenderPipeline,
    /// Annotations (text, arrows, lines, shapes)
    pub(super) annotations: Vec<Annotation>,
    /// Null policy for dataframe-backed numeric ingestion.
    pub(super) null_policy: NullPolicy,
    /// Deferred ingestion error captured during builder-style API calls.
    pub(super) pending_ingestion_error: Option<PendingIngestionError>,
    /// Group metadata used for grouped-series legend behavior.
    pub(super) series_groups: Vec<SeriesGroupMeta>,
    /// Monotonic group ID allocator for grouped-series builder scopes.
    pub(super) next_group_id: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SeriesGroupMeta {
    pub(super) id: usize,
    pub(super) label: Option<String>,
}

/// The first ingestion error a builder chain hit, plus a count of the ones that
/// followed it.
///
/// Builder methods cannot return `Result`, so the first failure is parked here
/// and re-raised by the terminal call. [`PlottingError`] is `Clone`, so this
/// holds the real error - variant identity and all - rather than a mirror enum
/// whose catch-all flattened unrecognised variants into a string.
#[derive(Clone, Debug)]
pub(crate) struct PendingIngestionError {
    first: PlottingError,
    additional_count: usize,
}

impl PendingIngestionError {
    pub(super) fn from_plotting_error(err: PlottingError) -> Self {
        Self {
            first: err,
            additional_count: 0,
        }
    }

    pub(super) fn record_additional_error(&mut self) {
        self.additional_count = self.additional_count.saturating_add(1);
    }

    pub(super) fn to_plotting_error(&self) -> PlottingError {
        if self.additional_count == 0 {
            return self.first.clone();
        }

        PlottingError::DataExtractionFailed {
            origin: "ruviz::plot-ingestion".to_string(),
            message: format!(
                "{} (and {} additional ingestion error{})",
                self.first,
                self.additional_count,
                if self.additional_count == 1 { "" } else { "s" }
            ),
        }
    }
}

/// Configuration for a single data series
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InsetAnchor {
    Auto,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    CenterLeft,
    CenterRight,
    Center,
    Custom { x_frac: f32, y_frac: f32 },
}

/// Inset placement for non-Cartesian series rendered inside a Cartesian plot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InsetLayout {
    pub anchor: InsetAnchor,
    pub width_frac: f32,
    pub height_frac: f32,
    pub margin_pt: f32,
}

impl InsetLayout {
    pub const DEFAULT_WIDTH_FRAC: f32 = 0.32;
    pub const DEFAULT_HEIGHT_FRAC: f32 = 0.32;
    pub const DEFAULT_MARGIN_PT: f32 = 12.0;

    pub fn auto() -> Self {
        Self {
            anchor: InsetAnchor::Auto,
            width_frac: Self::DEFAULT_WIDTH_FRAC,
            height_frac: Self::DEFAULT_HEIGHT_FRAC,
            margin_pt: Self::DEFAULT_MARGIN_PT,
        }
    }

    pub(super) fn normalized(self) -> Self {
        Self {
            anchor: self.anchor,
            width_frac: self.width_frac.clamp(0.12, 0.95),
            height_frac: self.height_frac.clamp(0.12, 0.95),
            margin_pt: self.margin_pt.max(0.0),
        }
    }
}

impl Default for InsetLayout {
    fn default() -> Self {
        Self::auto()
    }
}

/// The shared axis a [`MultiSeriesInput`] lays its value series along.
///
/// A grouped or stacked bar chart shares a *category* axis; a stacked area
/// chart shares a *numeric* one. Nothing else about the three differs, which is
/// why they share one input shape instead of one each.
#[derive(Clone, Debug, PartialEq)]
pub enum MultiSeriesAxis {
    /// One named category per slot, in slot order — `["Q1", "Q2", "Q3"]`.
    Categories(Vec<String>),
    /// One numeric x position per sample, in order.
    Positions(Vec<f64>),
}

impl MultiSeriesAxis {
    /// How many samples every value series is expected to carry.
    pub fn len(&self) -> usize {
        match self {
            Self::Categories(names) => names.len(),
            Self::Positions(values) => values.len(),
        }
    }

    /// Whether the axis carries no slots at all.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// N named value series sharing one axis.
///
/// This is the input shape of the crate's only *multi*-series plot types —
/// [`Plot::grouped_bar`](crate::core::Plot::grouped_bar),
/// [`Plot::stacked_bar`](crate::core::Plot::stacked_bar) and
/// [`Plot::stacked_area`](crate::core::Plot::stacked_area). Every other series
/// method takes one column of values; these take several, each with a name of
/// its own, because a grouped chart whose sub-series cannot be told apart in
/// the legend is not a grouped chart.
///
/// The name is carried here rather than on the builder because there are N of
/// them and `PlotBuilder::label` holds one. Each pair becomes an ordinary
/// series when the builder finalizes — its own `PlotSeries`, its own palette
/// slot, its own legend entry — so the series *count* is the only thing these
/// plot types change; the chain, the styling and the legend behave exactly as
/// they do for a line.
#[derive(Clone, Debug, PartialEq)]
pub struct MultiSeriesInput {
    /// The axis every value series is drawn against.
    pub axis: MultiSeriesAxis,
    /// `(series name, values)`, in draw order. An empty name means the series
    /// was not named, and [`PlotBuilder::label`](crate::core::PlotBuilder::label)
    /// supplies one for it.
    pub series: Vec<(String, Vec<f64>)>,
}

impl MultiSeriesInput {
    /// Total number of samples across every value series.
    pub fn point_count(&self) -> usize {
        self.series.iter().map(|(_, values)| values.len()).sum()
    }
}

/// Marker edge styling carried from a plot config to the renderer.
///
/// The fill colour is not known when a series is added (auto-palette series
/// resolve their colour at render time), so a `None` colour is stored as-is and
/// derived from the fill by [`MarkerEdge::resolve`] — the same
/// "`edge_color: None` means darken the fill" convention histogram and bar edges
/// use. The width is in **points**; the renderer converts it to device pixels,
/// so the rim is DPI-invariant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MarkerEdge {
    /// Explicit edge colour, or `None` to derive it from the fill.
    pub(super) color: Option<Color>,
    /// Edge width in points.
    pub(super) width: f32,
}

impl MarkerEdge {
    /// Resolve to the `(colour, width_in_points)` pair the renderer takes.
    ///
    /// Returns `None` for a non-positive width: an invisible edge is no edge.
    pub(super) fn resolve(self, theme: &Theme, fill: Color) -> Option<(Color, f32)> {
        crate::core::style_utils::StyleResolver::new(theme).patch_edge(fill, self.color, self.width)
    }
}

/// One style property: a static value **or** the live source it is sampled
/// from, held as a single owned field.
///
/// The two halves are mutually exclusive and this type owns that rule: storing
/// a static value retires the source, storing a reactive one retires the value.
/// Both halves are private, so there is no way to reach a state where a stale
/// source silently overwrites a value that was set after it.
///
/// A property with a legal range carries its own `normalize` function, supplied
/// once where the property is declared (see [`series_style_properties!`]).
/// Normalisation runs on a *static* value only: a reactive source is used
/// exactly as it samples, which is the behaviour these properties have always
/// had — the range is applied where the value is written, not where it is read.
pub(crate) struct Styled<T> {
    value: Option<T>,
    source: Option<ReactiveValue<T>>,
    normalize: fn(T) -> T,
}

impl<T> Styled<T> {
    /// An unset property that normalises static values with `normalize`.
    ///
    /// Only [`series_style_properties!`] calls this, so a property's range rule
    /// is fixed at its declaration and cannot vary by construction site.
    pub(crate) fn unset(normalize: fn(T) -> T) -> Self {
        Self {
            value: None,
            source: None,
            normalize,
        }
    }

    /// The static value, if this property is not driven by a live source.
    pub(crate) fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// The live source, if this property is driven by one.
    pub(crate) fn source(&self) -> Option<&ReactiveValue<T>> {
        self.source.as_ref()
    }

    /// Whether the property has been given a value or a source at all.
    pub(crate) fn is_set(&self) -> bool {
        self.value.is_some() || self.source.is_some()
    }

    /// Store `incoming`, keeping the two halves mutually exclusive.
    pub(crate) fn set(&mut self, incoming: ReactiveValue<T>) {
        match incoming {
            ReactiveValue::Static(value) => {
                self.value = Some((self.normalize)(value));
                self.source = None;
            }
            reactive => {
                self.value = None;
                self.source = Some(reactive);
            }
        }
    }

    /// Supply a fallback for a property whose static value is still unset.
    ///
    /// This deliberately looks at the *value* only, never the source: a plot
    /// type's config default fills the slot a user has not written, and a live
    /// source still wins at resolve time.
    pub(crate) fn or_value(&mut self, fallback: Option<T>) {
        if self.value.is_none() {
            self.value = fallback;
        }
    }

    /// Install a frame-resolved value, retiring the source that produced it.
    pub(crate) fn replace_resolved(&mut self, value: Option<T>) {
        self.value = value;
        self.source = None;
    }

    /// Retire the live source, keeping the static value.
    pub(crate) fn clear_source(&mut self) {
        self.source = None;
    }
}

impl<T: Clone> Styled<T> {
    /// The static value, cloned.
    pub(crate) fn cloned(&self) -> Option<T> {
        self.value.clone()
    }

    /// The static value, or `default` if unset.
    pub(crate) fn value_or(&self, default: T) -> T {
        self.value.clone().unwrap_or(default)
    }

    /// Sample this property for the frame at `time`.
    ///
    /// A live source wins over the static value; `cache` de-duplicates repeated
    /// samples of the same source within one frame.
    pub(crate) fn resolve(
        &self,
        time: f64,
        cache: &mut std::collections::HashMap<data::ReactiveSourceId, T>,
    ) -> Option<T> {
        let Some(source) = &self.source else {
            return self.value.clone();
        };

        if let Some(source_id) = source.source_id() {
            if let Some(value) = cache.get(&source_id) {
                return Some(value.clone());
            }
            let value = source.resolve(time);
            cache.insert(source_id, value.clone());
            return Some(value);
        }

        Some(source.resolve(time))
    }
}

impl<T: Send + Sync + 'static> Styled<T> {
    /// This property's live source with its value type erased, if it has one.
    fn erased_source(&self) -> Option<&dyn StyleSource> {
        match &self.source {
            Some(source) => Some(source),
            None => None,
        }
    }
}

impl<T: Clone> Clone for Styled<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            source: self.source.clone(),
            normalize: self.normalize,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Styled<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The normaliser is a function pointer with no useful rendering.
        f.debug_struct("Styled")
            .field("value", &self.value)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

/// Declare the reactive style properties every series carries — **once**.
///
/// This is the whole mechanism behind "adding a reactive style property is one
/// edit". The invocation below is the single list; from it this macro generates
///
/// - the [`SeriesStyleProps`] fields, so no struct literal anywhere names a
///   style property and none can be forgotten,
/// - `Default`, which is where each property's range rule is attached, so a
///   setter cannot forget to clamp,
/// - `sources`, whose destructure covers exactly the declared properties, and
/// - every traversal built on it: version collection, reactivity and temporality
///   tests, push subscription, and source clearing.
///
/// Before this existed each of those was written out property by property, and
/// a property left out of one of them produced a permanently stale plot with no
/// error at all. Now there is nowhere to leave it out.
///
/// # Adding a property
///
/// Add one line to the invocation below:
///
/// ```text
/// /// What the property means.
/// zorder: f32, normalize = |z| z.max(0.0);
/// ```
///
/// That is the whole edit. The field, its range rule, every traversal and the
/// traversal tests' fixtures all follow. A property whose type is new to this
/// list additionally needs a `TestStyleValue` impl — a compile error until you
/// write one, which is how a new type is made to say how it is tested.
///
/// Two things are deliberately *not* generated, because they are per-property
/// by nature rather than by duplication: reading the property in the renderer,
/// and folding it into `ResolvedSeriesStyle` (a struct literal, so also
/// compile-checked).
macro_rules! series_style_properties {
    (
        $(
            $(#[$doc:meta])*
            $name:ident: $ty:ty $(, normalize = $normalize:expr)?;
        )*
    ) => {
        /// The reactive style properties shared by [`PlotSeries`] and
        /// `SeriesStyle`.
        ///
        /// Both structs hold one of these rather than their own copy of the
        /// property list, so they cannot come to disagree about what a property
        /// is or what setting it means.
        ///
        /// Generated by [`series_style_properties!`]; see there for how to add
        /// a property.
        pub(crate) struct SeriesStyleProps {
            $(
                $(#[$doc])*
                pub(crate) $name: Styled<$ty>,
            )*
        }

        impl Default for SeriesStyleProps {
            fn default() -> Self {
                Self {
                    $(
                        // A property with no declared range keeps its value
                        // verbatim; `normalize = ..` above replaces the
                        // identity with the declared rule.
                        $name: Styled::unset({
                            #[allow(unused_assignments, unused_mut)]
                            let mut normalize: fn($ty) -> $ty = |value| value;
                            $( normalize = $normalize; )?
                            normalize
                        }),
                    )*
                }
            }
        }

        impl Clone for SeriesStyleProps {
            fn clone(&self) -> Self {
                Self { $( $name: self.$name.clone(), )* }
            }
        }

        impl fmt::Debug for SeriesStyleProps {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("SeriesStyleProps")
                    $( .field(stringify!($name), &self.$name) )*
                    .finish()
            }
        }

        impl SeriesStyleProps {
            /// The declared property names, in declaration order.
            #[cfg(test)]
            pub(crate) const NAMES: &'static [&'static str] = &[$(stringify!($name)),*];

            /// Every live style source, in declaration order.
            ///
            /// The destructure names exactly the generated fields, so this
            /// cannot fall behind the declaration list.
            fn sources(&self) -> impl Iterator<Item = &dyn StyleSource> {
                let Self { $( $name, )* } = self;
                [ $( $name.erased_source(), )* ].into_iter().flatten()
            }

            /// Retire every live source; frame-resolved values replace them.
            pub(crate) fn clear_sources(&mut self) {
                $( self.$name.clear_source(); )*
            }

            /// Whether any property has to be re-sampled at render time.
            pub(crate) fn has_reactive_sources(&self) -> bool {
                self.sources().any(StyleSource::is_reactive)
            }

            /// Whether any property varies with animation time.
            pub(crate) fn has_temporal_sources(&self) -> bool {
                self.sources().any(StyleSource::is_temporal)
            }

            /// Push each push-based source's version onto `versions`.
            pub(crate) fn collect_versions(&self, versions: &mut Vec<u64>) {
                for source in self.sources() {
                    if let Some(version) = source.current_version() {
                        versions.push(version);
                    }
                }
            }

            /// Register `callback` with every push-based source.
            pub(crate) fn subscribe(
                &self,
                callback: &SharedReactiveCallback,
                teardowns: &mut Vec<ReactiveTeardown>,
            ) {
                for source in self.sources() {
                    source.subscribe(Arc::clone(callback), teardowns);
                }
            }

            /// One `Self` per declared property, each with that property — and
            /// only that property — driven by a source of the given kind.
            ///
            /// Generated from the same list as the properties themselves, so
            /// the traversal tests cover a newly declared property without
            /// anyone remembering to extend a fixture.
            #[cfg(test)]
            pub(crate) fn each_property_sourced(
                kind: TestSourceKind,
            ) -> Vec<(&'static str, Self)> {
                vec![$({
                    let mut props = Self::default();
                    props.$name.set(<$ty as TestStyleValue>::source(kind));
                    (stringify!($name), props)
                }),*]
            }
        }
    };
}

series_style_properties! {
    /// Series color (`None` for auto-color from the palette).
    color: Color;
    /// Line width override, in points.
    line_width: f32, normalize = |width| width.max(0.1);
    /// Line dash pattern override.
    line_style: LineStyle;
    /// Marker shape for scatter-like series.
    marker_style: MarkerStyle;
    /// Marker size, in points.
    marker_size: f32, normalize = |size| size.max(0.1);
    /// Opacity, where 0.0 is transparent and 1.0 opaque.
    alpha: f32, normalize = |alpha| alpha.clamp(0.0, 1.0);
}

/// Which kind of reactive source a traversal test should attach.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum TestSourceKind {
    /// Push-based: has a version, marks the plot dirty when it changes.
    Push,
    /// Time-varying: has no version, must be re-sampled every frame.
    Temporal,
}

/// How to build a reactive source of a style property's value type, for tests.
///
/// [`SeriesStyleProps::each_property_sourced`] needs one value per declared
/// property type. Declaring a property of a type with no impl here is a compile
/// error, which is the point: a new property type has to say how it is tested.
#[cfg(test)]
pub(crate) trait TestStyleValue: Sized + Send + Sync + 'static {
    /// A representative value of this type.
    fn sample() -> Self;

    /// A reactive source of `kind` yielding [`Self::sample`].
    fn source(kind: TestSourceKind) -> ReactiveValue<Self>
    where
        Self: Clone,
    {
        match kind {
            TestSourceKind::Push => crate::data::Observable::new(Self::sample()).into(),
            TestSourceKind::Temporal => crate::data::Signal::new(|_| Self::sample()).into(),
        }
    }
}

#[cfg(test)]
impl TestStyleValue for Color {
    fn sample() -> Self {
        Color::RED
    }
}

#[cfg(test)]
impl TestStyleValue for f32 {
    fn sample() -> Self {
        0.5
    }
}

#[cfg(test)]
impl TestStyleValue for LineStyle {
    fn sample() -> Self {
        LineStyle::Dashed
    }
}

#[cfg(test)]
impl TestStyleValue for MarkerStyle {
    fn sample() -> Self {
        MarkerStyle::Square
    }
}

/// The read half of one style property, with its value type erased.
///
/// The six style properties carry four different `T`s, so a traversal that asks
/// all of them the same question needs a trait object. Before this existed each
/// question was written out six times over, and a property left out of
/// `collect_source_versions` produced a permanently stale plot with no error at
/// all.
pub(crate) trait StyleSource {
    /// Observable version, or `None` for a source that is not push-based.
    fn current_version(&self) -> Option<u64>;
    /// Whether this source has to be re-sampled at render time.
    fn is_reactive(&self) -> bool;
    /// Whether this source varies with animation time.
    fn is_temporal(&self) -> bool;
    /// Register `callback` for push updates, recording the teardown.
    fn subscribe(&self, callback: SharedReactiveCallback, teardowns: &mut Vec<ReactiveTeardown>);
}

impl<T: Send + Sync + 'static> StyleSource for ReactiveValue<T> {
    fn current_version(&self) -> Option<u64> {
        ReactiveValue::current_version(self)
    }

    fn is_reactive(&self) -> bool {
        ReactiveValue::is_reactive(self)
    }

    fn is_temporal(&self) -> bool {
        ReactiveValue::is_temporal(self)
    }

    fn subscribe(&self, callback: SharedReactiveCallback, teardowns: &mut Vec<ReactiveTeardown>) {
        self.subscribe_push_updates(callback, teardowns);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PlotSeries {
    /// Series type
    pub(super) series_type: SeriesType,
    /// Original paired streaming source for line/scatter streaming series.
    pub(super) streaming_source: Option<StreamingXY>,
    /// Series label for legend
    pub(super) label: Option<String>,
    /// The reactive style properties, declared once by
    /// [`series_style_properties!`].
    pub(super) props: SeriesStyleProps,
    /// Marker edge styling, or `None` for bare markers.
    pub(super) marker_edge: Option<MarkerEdge>,
    /// Whether this scatter uses plot-area density aggregation.
    /// Explicit per-series density choice; `None` lets fast mode decide.
    pub(super) density: Option<bool>,
    /// Optional Y error bar data (attached to series)
    pub(super) y_errors: Option<ErrorValues>,
    /// Optional X error bar data (attached to series)
    pub(super) x_errors: Option<ErrorValues>,
    /// Error bar configuration (cap size, line width, etc.)
    pub(super) error_config: Option<ErrorBarConfig>,
    /// Inset placement for non-Cartesian series in mixed plots.
    pub(super) inset_layout: Option<InsetLayout>,
    /// Optional group ID if this series was created inside `Plot::group(...)`.
    pub(super) group_id: Option<usize>,
    /// Frame-resolved colors for multi-series radar payloads.
    pub(super) resolved_radar_colors: Option<Arc<[Color]>>,
    /// Whether the series is drawn. A hidden series keeps its index, its
    /// palette slot, and a dimmed legend entry; it is skipped by rendering
    /// and hit testing and does not affect axis bounds.
    pub(crate) visible: bool,
}

impl PlotSeries {
    pub(super) fn to_legend_item_with_label(
        &self,
        label: String,
        default_color: Color,
        theme: &Theme,
    ) -> Option<LegendItem> {
        if label.is_empty() {
            return None;
        }

        self.build_legend_item(label, default_color, theme)
    }

    /// Create a LegendItem from this series
    ///
    /// Returns None if the series has no label
    pub(super) fn to_legend_item(&self, default_color: Color, theme: &Theme) -> Option<LegendItem> {
        let label = self.label.as_ref()?;
        self.build_legend_item(label.clone(), default_color, theme)
    }

    pub(super) fn build_legend_item(
        &self,
        label: String,
        default_color: Color,
        theme: &Theme,
    ) -> Option<LegendItem> {
        let color = self.color_with_alpha(default_color);
        let line_width = self.props.line_width.value_or(theme.line_width);
        let line_style = self.props.line_style.value_or(LineStyle::Solid);
        let marker_style = self.props.marker_style.value_or(MarkerStyle::Circle);
        let marker_size = self.props.marker_size.value_or(6.0);

        // The key carries the *same* resolved rim the renderer strokes the
        // markers with, so a legend swatch cannot disagree with its series.
        let marker_edge = self.marker_edge.and_then(|edge| edge.resolve(theme, color));

        let item_type = match &self.series_type {
            SeriesType::Line { .. } => {
                if self.props.marker_style.is_set() {
                    LegendItemType::LineMarker {
                        line_style,
                        line_width,
                        marker: marker_style,
                        marker_size,
                        marker_edge,
                    }
                } else {
                    LegendItemType::Line {
                        style: line_style,
                        width: line_width,
                    }
                }
            }
            SeriesType::Scatter { .. } => LegendItemType::Scatter {
                marker: marker_style,
                size: marker_size,
                edge: marker_edge,
            },
            // Patch keys carry the *same* resolved edge the renderer strokes the
            // patch with, so the key and the plot cannot disagree. Each config
            // owns that resolution (all of them funnel through
            // `StyleResolver::patch_edge`) - never re-derive it here.
            SeriesType::Bar { config, .. } => LegendItemType::Bar {
                edge: config.resolved_edge(theme, color),
            },
            SeriesType::ErrorBars { .. } | SeriesType::ErrorBarsXY { .. } => {
                LegendItemType::ErrorBar
            }
            SeriesType::Histogram {
                config, prepared, ..
            } => LegendItemType::Histogram {
                edge: match prepared {
                    // Binned data is the exact thing that gets drawn; prefer it.
                    Some(data) => data.resolved_edge(theme, color),
                    None => config.resolved_edge(theme, color),
                },
            },
            SeriesType::BoxPlot { config, .. } => LegendItemType::Bar {
                edge: config.resolved_edge(theme, color),
            },
            SeriesType::Heatmap { .. } => return None,
            SeriesType::Kde { .. }
            | SeriesType::Ecdf { .. }
            | SeriesType::Polar { .. }
            | SeriesType::Quiver { .. } => LegendItemType::Line {
                style: line_style,
                width: line_width,
            },
            // Compute-only types are not all stroked: a strip or swarm cloud is
            // markers and a hexbin is a colour scale, so each says which mark
            // stands for it rather than all of them inheriting the line key.
            SeriesType::Computed { data } => match data.legend_key() {
                crate::plots::traits::LegendKey::Line => LegendItemType::Line {
                    style: line_style,
                    width: line_width,
                },
                crate::plots::traits::LegendKey::Marker => LegendItemType::Scatter {
                    marker: marker_style,
                    size: marker_size,
                    edge: marker_edge,
                },
                // The swatch is stroked with the edge the series' own patches
                // carry, resolved through the one filled-patch rule — a flat
                // key for an outlined bar misdescribes the picture.
                crate::plots::traits::LegendKey::Patch => LegendItemType::Bar {
                    edge: data.patch_edge_spec().and_then(|(explicit, width)| {
                        crate::core::style_utils::StyleResolver::new(theme)
                            .patch_edge(color, explicit, width)
                    }),
                },
                crate::plots::traits::LegendKey::None => return None,
            },
            // A violin body is outlined in its configured line colour — or the
            // fill darkened, the shared edge rule — so its key is too.
            SeriesType::Violin { data } => LegendItemType::Bar {
                edge: (data.config.line_width > 0.0).then(|| {
                    (
                        crate::core::style_utils::StyleResolver::new(theme)
                            .edge_color(color, data.config.line_color),
                        data.config.line_width,
                    )
                }),
            },
            // Flat keys that tell the truth: a boxen's outline is stroked in
            // the same colour as its base fill, so an edge on the swatch would
            // be invisible, and pie wedges carry no edge at all.
            SeriesType::Boxen { .. } | SeriesType::Pie { .. } => LegendItemType::Bar { edge: None },
            SeriesType::Contour { .. } => return None,
            SeriesType::Radar { .. } => LegendItemType::Area {
                edge_color: Some(color),
            },
        };

        let has_error_bars = self.y_errors.is_some() || self.x_errors.is_some();

        Some(LegendItem {
            label,
            color,
            item_type,
            has_error_bars,
            series_indices: Vec::new(),
            dimmed: !self.visible,
        })
    }

    /// Create multiple LegendItems from this series (for Radar series that contain multiple internal series)
    ///
    /// Returns a Vec of legend items, expanding radar series into individual entries per data series.
    pub(super) fn to_legend_items(&self, base_color_idx: usize, theme: &Theme) -> Vec<LegendItem> {
        match &self.series_type {
            SeriesType::Radar { data } => {
                // For radar charts, create a legend item for each internal series
                // Use Area type to show filled swatches matching the filled polygon style
                data.series
                    .iter()
                    .enumerate()
                    .map(|(idx, radar_series)| {
                        let color = self
                            .resolved_radar_colors
                            .as_ref()
                            .and_then(|colors| colors.get(idx).copied())
                            .or_else(|| {
                                data.config
                                    .colors
                                    .as_ref()
                                    .and_then(|colors| colors.get(idx).copied())
                                    .filter(|color| *color != Color::TRANSPARENT)
                            })
                            .unwrap_or_else(|| theme.get_color(base_color_idx + idx));
                        // Use a more visible alpha for legend swatches (0.6 instead of fill_alpha)
                        // This ensures legend items are clearly visible while still showing fill style
                        let series_alpha = self.props.alpha.value_or(1.0);
                        let color_alpha = f32::from(color.a) / 255.0;
                        let edge_color = color.with_alpha(color_alpha * series_alpha);
                        let fill_color = color.with_alpha(color_alpha * 0.6 * series_alpha);
                        LegendItem {
                            label: radar_series.label.clone(),
                            color: fill_color,
                            item_type: LegendItemType::Area {
                                edge_color: Some(edge_color),
                            },
                            has_error_bars: false,
                            series_indices: Vec::new(),
                            dimmed: !self.visible,
                        }
                    })
                    .collect()
            }
            _ => {
                // For other series types, use the single-item method
                let default_color = theme.get_color(base_color_idx);
                self.to_legend_item(default_color, theme)
                    .into_iter()
                    .collect()
            }
        }
    }

    pub(super) fn collect_source_versions(&self, versions: &mut Vec<u64>) {
        if let Some(stream) = &self.streaming_source {
            stream.refresh_legacy_lanes();
            versions.push(stream.version());
            versions.push(stream.x().version());
            versions.push(stream.y().version());
        } else {
            self.series_type.collect_source_versions(versions);
        }
        self.props.collect_versions(versions);
    }

    pub(super) fn is_reactive(&self) -> bool {
        self.series_type.is_reactive() || self.has_reactive_style_sources()
    }

    pub(super) fn has_temporal_sources(&self) -> bool {
        self.series_type.has_temporal_sources() || self.props.has_temporal_sources()
    }

    pub(super) fn has_reactive_style_sources(&self) -> bool {
        self.props.has_reactive_sources()
    }

    /// Clone this series for a resolved frame, shedding the static data payload.
    ///
    /// Spelled out field by field on purpose: `Self { series_type: .., ..self.clone() }`
    /// would clone the static `Vec<f64>` lanes only to throw them away, which is
    /// a per-frame copy of the whole dataset in an animation. A struct literal
    /// is exhaustively checked anyway, so a new field cannot be dropped here
    /// silently — it simply will not compile.
    pub(super) fn clone_for_resolved_frame(&self) -> Self {
        Self {
            series_type: self.series_type.clone_without_static_values(),
            streaming_source: self.streaming_source.clone(),
            label: self.label.clone(),
            props: self.props.clone(),
            marker_edge: self.marker_edge,
            density: self.density,
            y_errors: self.y_errors.clone(),
            x_errors: self.x_errors.clone(),
            error_config: self.error_config.clone(),
            inset_layout: self.inset_layout,
            group_id: self.group_id,
            resolved_radar_colors: self.resolved_radar_colors.clone(),
            visible: self.visible,
        }
    }

    pub(super) fn mark_rendered_sources(&self) {
        if let Some(stream) = &self.streaming_source {
            stream.mark_rendered();
        } else {
            self.series_type.mark_rendered_sources();
        }
    }

    pub(super) fn mark_unpaired_streaming_sources_rendered(&self) {
        if self.streaming_source.is_none() {
            self.series_type.mark_rendered_sources();
        }
    }

    pub(super) fn has_live_streaming_pair(&self) -> bool {
        self.streaming_source.is_some()
            && matches!(
                &self.series_type,
                SeriesType::Line { x_data, y_data }
                    | SeriesType::Scatter { x_data, y_data }
                    if matches!(x_data, PlotData::Streaming(_))
                        && matches!(y_data, PlotData::Streaming(_))
            )
    }

    pub(super) fn resolve_for_render(&self, time: f64) -> Result<ResolvedSeries<'_>> {
        if self.has_live_streaming_pair() {
            let snapshot = self
                .streaming_source
                .as_ref()
                .expect("live streaming pair must retain its source")
                .snapshot();
            return Ok(match &self.series_type {
                SeriesType::Line { .. } => ResolvedSeries::Line {
                    x: ResolvedData::shared(Arc::from(snapshot.x())),
                    y: ResolvedData::shared(Arc::from(snapshot.y())),
                },
                SeriesType::Scatter { .. } => ResolvedSeries::Scatter {
                    x: ResolvedData::shared(Arc::from(snapshot.x())),
                    y: ResolvedData::shared(Arc::from(snapshot.y())),
                },
                _ => unreachable!("live paired source is only used by line/scatter"),
            });
        }
        self.series_type.resolve_for_render(time)
    }

    pub(super) fn subscribe_push_updates(
        &self,
        callback: &SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) {
        if let Some(stream) = &self.streaming_source {
            let stream = stream.clone();
            let callback = Arc::clone(callback);
            stream.refresh_legacy_lanes();
            let observed_versions = Arc::new(std::sync::Mutex::new((
                stream.version(),
                stream.x().version(),
                stream.y().version(),
            )));
            let make_callback = |stream: crate::data::StreamingXY| {
                let callback = Arc::clone(&callback);
                let observed_versions = Arc::clone(&observed_versions);
                move || {
                    stream.refresh_legacy_lanes();
                    let current = (stream.version(), stream.x().version(), stream.y().version());
                    let mut observed = observed_versions
                        .lock()
                        .expect("Streaming subscription version lock poisoned");
                    if *observed != current {
                        *observed = current;
                        drop(observed);
                        callback();
                    }
                }
            };
            let paired_id = stream.subscribe_paired(make_callback(stream.clone()));
            let x_id = stream.x().subscribe(make_callback(stream.clone()));
            let y_id = stream.y().subscribe(make_callback(stream.clone()));
            teardowns.push(Box::new(move || {
                stream.unsubscribe_paired(paired_id);
                stream.x().unsubscribe(x_id);
                stream.y().unsubscribe(y_id);
            }));
        } else {
            self.series_type.subscribe_push_updates(callback, teardowns);
        }
        self.props.subscribe(callback, teardowns);
    }

    pub(super) fn color_with_alpha(&self, default_color: Color) -> Color {
        let color = self.props.color.value_or(default_color);
        let alpha = self.props.alpha.value_or(1.0).clamp(0.0, 1.0);
        color.with_alpha((f32::from(color.a) / 255.0) * alpha)
    }
}

/// Types of plot series
#[derive(Clone, Debug)]
pub(crate) enum SeriesType {
    Line {
        x_data: PlotData,
        y_data: PlotData,
    },
    Scatter {
        x_data: PlotData,
        y_data: PlotData,
    },
    Bar {
        categories: Vec<String>,
        values: PlotData,
        /// Geometry and edge styling for the bars.
        ///
        /// Carried on the variant (like `Histogram`) because the renderer needs
        /// the edge colour and width, which no other field on `PlotSeries`
        /// records.
        config: crate::plots::basic::BarConfig,
    },
    ErrorBars {
        x_data: PlotData,
        y_data: PlotData,
        y_errors: PlotData,
    },
    ErrorBarsXY {
        x_data: PlotData,
        y_data: PlotData,
        x_errors: PlotData,
        y_errors: PlotData,
    },
    Histogram {
        data: PlotData,
        config: crate::plots::histogram::HistogramConfig,
        prepared: Option<crate::plots::histogram::HistogramData>,
    },
    BoxPlot {
        data: PlotData,
        config: crate::plots::boxplot::BoxPlotConfig,
    },
    Heatmap {
        data: Arc<crate::plots::heatmap::HeatmapData>,
    },
    /// KDE (Kernel Density Estimation) plot
    Kde {
        data: Arc<crate::plots::KdeData>,
    },
    /// ECDF (Empirical Cumulative Distribution Function) plot
    Ecdf {
        data: Arc<crate::plots::EcdfData>,
    },
    /// Violin plot
    Violin {
        data: Arc<crate::plots::ViolinData>,
    },
    /// Boxen (Letter-Value) plot
    Boxen {
        data: Arc<crate::plots::BoxenData>,
    },
    /// Contour plot
    Contour {
        data: Arc<crate::plots::continuous::contour::ContourPlotData>,
    },
    /// Pie chart
    Pie {
        data: Arc<crate::plots::composition::pie::PieData>,
    },
    /// Radar chart
    Radar {
        data: Arc<crate::plots::polar::radar::RadarPlotData>,
    },
    /// Polar plot
    Polar {
        data: Arc<crate::plots::polar::polar_plot::PolarPlotData>,
    },
    /// Quiver vector field plot
    Quiver {
        data: Arc<crate::plots::QuiverPlotData>,
    },
    /// Any plot type that ships finished geometry behind
    /// [`ComputedSeries`](crate::plots::traits::ComputedSeries).
    ///
    /// One variant, not one per plot type. `SeriesType` is matched exhaustively
    /// in eleven places, so five compute-only plot types would have cost forty
    /// match arms and five more things to keep in step. Rug, strip, swarm,
    /// hexbin and dendrogram all arrive here, and the next such plot type needs
    /// no edit to this enum at all.
    Computed {
        data: Arc<dyn crate::plots::traits::ComputedSeries>,
    },
}

impl SeriesType {
    pub(crate) fn supports_interactive_surface_fast_path(&self) -> bool {
        matches!(
            self,
            SeriesType::Line { .. } | SeriesType::Scatter { .. } | SeriesType::Heatmap { .. }
        )
    }

    /// Check if this series contains any reactive data
    pub fn is_reactive(&self) -> bool {
        match self {
            SeriesType::Line { x_data, y_data } => x_data.is_reactive() || y_data.is_reactive(),
            SeriesType::Scatter { x_data, y_data } => x_data.is_reactive() || y_data.is_reactive(),
            SeriesType::Bar { values, .. } => values.is_reactive(),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => x_data.is_reactive() || y_data.is_reactive() || y_errors.is_reactive(),
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                x_data.is_reactive()
                    || y_data.is_reactive()
                    || x_errors.is_reactive()
                    || y_errors.is_reactive()
            }
            SeriesType::Histogram { data, .. } => data.is_reactive(),
            SeriesType::BoxPlot { data, .. } => data.is_reactive(),
            // Other types use their own data structures, not PlotData
            _ => false,
        }
    }

    pub(super) fn has_temporal_sources(&self) -> bool {
        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                x_data.is_temporal() || y_data.is_temporal()
            }
            SeriesType::Bar { values, .. } => values.is_temporal(),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => x_data.is_temporal() || y_data.is_temporal() || y_errors.is_temporal(),
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                x_data.is_temporal()
                    || y_data.is_temporal()
                    || x_errors.is_temporal()
                    || y_errors.is_temporal()
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                data.is_temporal()
            }
            _ => false,
        }
    }

    pub(super) fn collect_source_versions(&self, versions: &mut Vec<u64>) {
        let push = |data: &PlotData, versions: &mut Vec<u64>| {
            if let Some(version) = data.current_version() {
                versions.push(version);
            }
        };

        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                push(x_data, versions);
                push(y_data, versions);
            }
            SeriesType::Bar { values, .. } => push(values, versions),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                push(x_data, versions);
                push(y_data, versions);
                push(y_errors, versions);
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                push(x_data, versions);
                push(y_data, versions);
                push(x_errors, versions);
                push(y_errors, versions);
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                push(data, versions);
            }
            _ => {}
        }
    }

    pub(super) fn mark_rendered_sources(&self) {
        let mark = |data: &PlotData| data.mark_rendered();

        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                mark(x_data);
                mark(y_data);
            }
            SeriesType::Bar { values, .. } => mark(values),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                mark(x_data);
                mark(y_data);
                mark(y_errors);
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                mark(x_data);
                mark(y_data);
                mark(x_errors);
                mark(y_errors);
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                mark(data);
            }
            _ => {}
        }
    }

    pub(super) fn subscribe_push_updates(
        &self,
        callback: &SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) {
        let subscribe = |data: &PlotData, teardowns: &mut Vec<ReactiveTeardown>| {
            data.subscribe_push_updates(Arc::clone(callback), teardowns);
        };

        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                subscribe(x_data, teardowns);
                subscribe(y_data, teardowns);
            }
            SeriesType::Bar { values, .. } => subscribe(values, teardowns),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                subscribe(x_data, teardowns);
                subscribe(y_data, teardowns);
                subscribe(y_errors, teardowns);
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                subscribe(x_data, teardowns);
                subscribe(y_data, teardowns);
                subscribe(x_errors, teardowns);
                subscribe(y_errors, teardowns);
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                subscribe(data, teardowns);
            }
            _ => {}
        }
    }

    pub(super) fn clone_without_static_values(&self) -> SeriesType {
        match self {
            SeriesType::Line { x_data, y_data } => SeriesType::Line {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
            },
            SeriesType::Scatter { x_data, y_data } => SeriesType::Scatter {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
            },
            SeriesType::Bar {
                categories,
                values,
                config,
            } => SeriesType::Bar {
                categories: categories.clone(),
                values: values.clone_without_static_values(),
                config: config.clone(),
            },
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => SeriesType::ErrorBars {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
                y_errors: y_errors.clone_without_static_values(),
            },
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => SeriesType::ErrorBarsXY {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
                x_errors: x_errors.clone_without_static_values(),
                y_errors: y_errors.clone_without_static_values(),
            },
            SeriesType::Histogram {
                data,
                config,
                prepared,
            } => SeriesType::Histogram {
                data: data.clone_without_static_values(),
                config: config.clone(),
                prepared: prepared.clone(),
            },
            SeriesType::BoxPlot { data, config } => SeriesType::BoxPlot {
                data: data.clone_without_static_values(),
                config: config.clone(),
            },
            other => other.clone(),
        }
    }

    /// Resolve all PlotData in this series to static Vec<f64> at the given time
    ///
    /// Returns a new SeriesType with all PlotData converted to PlotData::Static
    pub fn resolve(&self, time: f64) -> SeriesType {
        match self {
            SeriesType::Line { x_data, y_data } => SeriesType::Line {
                x_data: PlotData::Static(x_data.resolve(time).into()),
                y_data: PlotData::Static(y_data.resolve(time).into()),
            },
            SeriesType::Scatter { x_data, y_data } => SeriesType::Scatter {
                x_data: PlotData::Static(x_data.resolve(time).into()),
                y_data: PlotData::Static(y_data.resolve(time).into()),
            },
            SeriesType::Bar {
                categories,
                values,
                config,
            } => SeriesType::Bar {
                categories: categories.clone(),
                values: PlotData::Static(values.resolve(time).into()),
                config: config.clone(),
            },
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => SeriesType::ErrorBars {
                x_data: PlotData::Static(x_data.resolve(time).into()),
                y_data: PlotData::Static(y_data.resolve(time).into()),
                y_errors: PlotData::Static(y_errors.resolve(time).into()),
            },
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => SeriesType::ErrorBarsXY {
                x_data: PlotData::Static(x_data.resolve(time).into()),
                y_data: PlotData::Static(y_data.resolve(time).into()),
                x_errors: PlotData::Static(x_errors.resolve(time).into()),
                y_errors: PlotData::Static(y_errors.resolve(time).into()),
            },
            SeriesType::Histogram {
                data,
                config,
                prepared,
            } => {
                let resolved_data = data.resolve(time);
                let prepared = prepared.clone().or_else(|| {
                    crate::plots::histogram::calculate_histogram(&resolved_data, config).ok()
                });
                SeriesType::Histogram {
                    data: PlotData::Static(resolved_data.into()),
                    config: config.clone(),
                    prepared,
                }
            }
            SeriesType::BoxPlot { data, config } => SeriesType::BoxPlot {
                data: PlotData::Static(data.resolve(time).into()),
                config: config.clone(),
            },
            // Other types don't use PlotData - clone as-is
            other => other.clone(),
        }
    }

    /// Get resolved x_data as Vec<f64> for series that carry x-data.
    #[inline]
    pub fn try_x_data_resolved(&self, time: f64) -> Option<Vec<f64>> {
        match self {
            SeriesType::Line { x_data, .. }
            | SeriesType::Scatter { x_data, .. }
            | SeriesType::ErrorBars { x_data, .. }
            | SeriesType::ErrorBarsXY { x_data, .. } => Some(x_data.resolve(time)),
            _ => None,
        }
    }

    /// Get resolved y_data as Vec<f64> for series that carry y-data.
    #[inline]
    pub fn try_y_data_resolved(&self, time: f64) -> Option<Vec<f64>> {
        match self {
            SeriesType::Line { y_data, .. }
            | SeriesType::Scatter { y_data, .. }
            | SeriesType::ErrorBars { y_data, .. }
            | SeriesType::ErrorBarsXY { y_data, .. } => Some(y_data.resolve(time)),
            _ => None,
        }
    }

    /// Get resolved x_data as Vec<f64> (panics if not Line/Scatter/ErrorBars).
    #[deprecated(note = "Use try_x_data_resolved() for non-panicking behavior")]
    #[inline]
    pub fn x_data_resolved(&self, time: f64) -> Vec<f64> {
        self.try_x_data_resolved(time)
            .expect("x_data not available for this series type")
    }

    /// Get resolved y_data as Vec<f64> (panics if not Line/Scatter/ErrorBars).
    #[deprecated(note = "Use try_y_data_resolved() for non-panicking behavior")]
    #[inline]
    pub fn y_data_resolved(&self, time: f64) -> Vec<f64> {
        self.try_y_data_resolved(time)
            .expect("y_data not available for this series type")
    }

    pub(super) fn histogram_data_at(
        &self,
        time: f64,
    ) -> Result<crate::plots::histogram::HistogramData> {
        match self {
            SeriesType::Histogram {
                data,
                config,
                prepared,
            } => {
                let resolved = data.resolve_cow(time);
                if resolved.is_empty() {
                    return Err(PlottingError::EmptyDataSet);
                }
                PlottingError::validate_data(resolved.as_ref())?;

                if let Some(prepared) = prepared {
                    return Ok(prepared.clone());
                }

                crate::plots::histogram::calculate_histogram(&resolved.as_ref(), config).map_err(
                    |error| {
                        PlottingError::RenderError(format!("Histogram calculation failed: {error}"))
                    },
                )
            }
            _ => Err(PlottingError::RenderError(
                "histogram_data_at called for non-histogram series".to_string(),
            )),
        }
    }
}

#[derive(Clone)]
pub(crate) enum ResolvedData<'a> {
    Cow(Cow<'a, [f64]>),
    Shared(Arc<[f64]>),
}

impl<'a> ResolvedData<'a> {
    fn from_cow(data: Cow<'a, [f64]>) -> Self {
        match data {
            Cow::Borrowed(data) => Self::Cow(Cow::Borrowed(data)),
            Cow::Owned(data) => Self::Shared(Arc::from(data)),
        }
    }

    pub(super) fn shared(data: Arc<[f64]>) -> Self {
        Self::Shared(data)
    }

    pub(super) fn shared_arc(&self) -> Option<Arc<[f64]>> {
        match self {
            Self::Cow(Cow::Borrowed(_)) => None,
            Self::Cow(Cow::Owned(data)) => Some(Arc::from(data.clone())),
            Self::Shared(data) => Some(Arc::clone(data)),
        }
    }
}

impl AsRef<[f64]> for ResolvedData<'_> {
    fn as_ref(&self) -> &[f64] {
        match self {
            Self::Cow(data) => data.as_ref(),
            Self::Shared(data) => data.as_ref(),
        }
    }
}

impl std::ops::Deref for ResolvedData<'_> {
    type Target = [f64];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

#[derive(Clone)]
pub(crate) enum ResolvedSeries<'a> {
    Line {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
    },
    Scatter {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
    },
    Bar {
        categories: &'a [String],
        values: ResolvedData<'a>,
        config: &'a crate::plots::basic::BarConfig,
    },
    ErrorBars {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
        y_errors: ResolvedData<'a>,
    },
    ErrorBarsXY {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
        x_errors: ResolvedData<'a>,
        y_errors: ResolvedData<'a>,
    },
    Histogram {
        data: crate::plots::histogram::HistogramData,
    },
    BoxPlot {
        data: ResolvedData<'a>,
        config: BoxPlotConfig,
    },
    Other(&'a SeriesType),
}

pub(crate) struct ResolvedStreamingPair {
    pub(super) source: StreamingXY,
    pub(super) x: Arc<[f64]>,
    pub(super) y: Arc<[f64]>,
    pub(super) watermark: crate::data::observable::StreamingXYRenderWatermark,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedSeriesStyle {
    pub(super) color: Color,
    pub(super) line_width: Option<f32>,
    pub(super) line_style: LineStyle,
    pub(super) marker_style: Option<MarkerStyle>,
    pub(super) marker_size: Option<f32>,
    pub(super) alpha: f32,
    pub(super) radar_colors: Option<Arc<[Color]>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedStyle {
    pub(super) theme: Theme,
    pub(super) config: PlotConfig,
    pub(super) grid_style: GridStyle,
    pub(super) legend: Legend,
    pub(super) series: Vec<ResolvedSeriesStyle>,
}

pub(crate) struct ResolvedFrame<'a> {
    pub(super) series: Vec<ResolvedSeries<'a>>,
    pub(super) style: ResolvedStyle,
    pub(super) title: Option<String>,
    pub(super) xlabel: Option<String>,
    pub(super) ylabel: Option<String>,
    pub(super) streaming_acknowledgements: Vec<crate::data::StreamingBuffer<f64>>,
    pub(super) paired_acknowledgements: Vec<ResolvedStreamingPair>,
}

impl ResolvedFrame<'_> {
    pub(super) fn acknowledge_rendered(&self, _live_plot: &Plot) {
        for stream in &self.streaming_acknowledgements {
            stream.mark_rendered();
        }
        for stream in &self.paired_acknowledgements {
            stream
                .source
                .mark_rendered_through(stream.watermark.sequence());
        }
    }
}

impl SeriesType {
    pub(super) fn resolve_for_render(&self, time: f64) -> Result<ResolvedSeries<'_>> {
        Ok(match self {
            SeriesType::Line { x_data, y_data } => ResolvedSeries::Line {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
            },
            SeriesType::Scatter { x_data, y_data } => ResolvedSeries::Scatter {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
            },
            SeriesType::Bar {
                categories,
                values,
                config,
            } => ResolvedSeries::Bar {
                categories,
                values: ResolvedData::from_cow(values.resolve_cow(time)),
                config,
            },
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => ResolvedSeries::ErrorBars {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
                y_errors: ResolvedData::from_cow(y_errors.resolve_cow(time)),
            },
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => ResolvedSeries::ErrorBarsXY {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
                x_errors: ResolvedData::from_cow(x_errors.resolve_cow(time)),
                y_errors: ResolvedData::from_cow(y_errors.resolve_cow(time)),
            },
            SeriesType::Histogram { .. } => ResolvedSeries::Histogram {
                data: self.histogram_data_at(time)?,
            },
            SeriesType::BoxPlot { data, config } => ResolvedSeries::BoxPlot {
                data: ResolvedData::from_cow(data.resolve_cow(time)),
                config: config.clone(),
            },
            other => ResolvedSeries::Other(other),
        })
    }
}

/// Legend configuration (legacy, for backward compatibility)
#[derive(Clone, Debug)]
pub(crate) struct LegendConfig {
    /// Whether to show legend
    pub(crate) enabled: bool,
    /// Legend position
    pub(crate) position: LegendPosition,
    /// Font size override in typographic points.
    pub(crate) font_size: Option<f32>,
    /// Corner radius for rounded corners
    pub(crate) corner_radius: Option<f32>,
    /// Number of columns (1 = vertical, >1 = horizontal/multi-column)
    pub(crate) columns: Option<usize>,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: LegendPosition::UpperRight,
            font_size: None,
            corner_radius: None,
            columns: None,
        }
    }
}

impl LegendConfig {
    /// Convert to new Legend type for rendering
    pub(super) fn to_legend(&self, default_font_size: f32) -> Legend {
        let mut legend = Legend {
            enabled: self.enabled,
            position: self.position,
            font_size: self.font_size.unwrap_or(default_font_size),
            ..Legend::default()
        };
        if let Some(radius) = self.corner_radius {
            legend.style.corner_radius = radius;
        }
        if let Some(cols) = self.columns {
            legend.columns = cols.max(1);
        }
        legend
    }
}

// NOTE: GridConfig has been replaced by the unified GridStyle from core module.
// See `grid_style: GridStyle` field in Plot struct for grid configuration.

/// Tick configuration for axes
#[derive(Clone, Debug)]
pub(crate) struct TickConfig {
    /// Whether tick marks and tick labels are rendered
    pub(crate) enabled: bool,
    /// Direction ticks point (inside or outside)
    pub(crate) direction: TickDirection,
    /// Which plot borders render tick marks
    pub(crate) sides: TickSides,
    /// Number of major ticks on X axis
    pub(crate) major_ticks_x: usize,
    /// Number of minor ticks between major ticks on X axis
    pub(crate) minor_ticks_x: usize,
    /// Number of major ticks on Y axis
    pub(crate) major_ticks_y: usize,
    /// Number of minor ticks between major ticks on Y axis
    pub(crate) minor_ticks_y: usize,
    /// Grid display mode
    pub(crate) grid_mode: GridMode,
    /// How the x tick labels are oriented when they would collide.
    pub(crate) xtick_rotation: crate::render::skia::XTickRotation,
}

impl Default for TickConfig {
    fn default() -> Self {
        TickConfig {
            enabled: true,
            direction: TickDirection::Inside,
            sides: TickSides::all(),
            major_ticks_x: 10,
            minor_ticks_x: 0,
            major_ticks_y: 8,
            minor_ticks_y: 0,
            grid_mode: GridMode::MajorOnly,
            xtick_rotation: crate::render::skia::XTickRotation::Auto,
        }
    }
}

#[cfg(test)]
mod marker_edge_tests {
    use super::*;

    #[test]
    fn test_marker_edge_derives_its_colour_from_the_fill() {
        let theme = Theme::default();
        let fill = Color::from_rgb(31, 119, 180);
        let edge = MarkerEdge {
            color: None,
            width: 0.5,
        };

        assert_eq!(
            edge.resolve(&theme, fill),
            Some((fill.darken(0.3), 0.5)),
            "a marker edge with no explicit colour must darken the fill it bounds"
        );
    }

    #[test]
    fn test_marker_edge_keeps_an_explicit_colour() {
        let theme = Theme::default();
        let edge = MarkerEdge {
            color: Some(Color::BLACK),
            width: 1.5,
        };

        assert_eq!(
            edge.resolve(&theme, Color::from_rgb(31, 119, 180)),
            Some((Color::BLACK, 1.5)),
            "an explicit marker edge colour must not be recoloured"
        );
    }

    #[test]
    fn test_marker_edge_of_zero_width_resolves_to_nothing() {
        let theme = Theme::default();
        let edge = MarkerEdge {
            color: Some(Color::BLACK),
            width: 0.0,
        };

        assert_eq!(
            edge.resolve(&theme, Color::from_rgb(31, 119, 180)),
            None,
            "a zero-width edge is no edge, not a hairline"
        );
    }
}

/// The safety net under `PlotSeries::style_sources`.
///
/// Every reactive style property has to be visible to *every* traversal. The
/// failure this guards against is silent: a property left out of
/// `collect_source_versions` renders once and then never updates again, with no
/// error anywhere. These tests set each property in turn and assert that all
/// four traversals notice, so a seventh property that is only half-wired fails
/// here instead of in a user's animation.
#[cfg(test)]
mod reactive_style_tests {
    use super::*;
    use crate::data::Observable;

    fn bare_series() -> PlotSeries {
        PlotSeries {
            series_type: SeriesType::Line {
                x_data: PlotData::Static(vec![0.0, 1.0].into()),
                y_data: PlotData::Static(vec![0.0, 1.0].into()),
            },
            streaming_source: None,
            label: None,
            props: SeriesStyleProps::default(),
            marker_edge: None,
            density: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
            visible: true,
        }
    }

    /// One series per *declared* style property, each carrying a source of
    /// `kind` on that property alone.
    ///
    /// The property list comes from [`series_style_properties!`], not from a
    /// hand-written fixture, so a newly declared property is covered by every
    /// test below the moment it is declared.
    fn series_per_property(kind: TestSourceKind) -> Vec<(&'static str, PlotSeries)> {
        SeriesStyleProps::each_property_sourced(kind)
            .into_iter()
            .map(|(name, props)| {
                let mut series = bare_series();
                series.props = props;
                (name, series)
            })
            .collect()
    }

    /// The fixtures really do cover every declared property.
    #[test]
    fn the_traversal_fixtures_cover_every_declared_property() {
        let covered: Vec<&str> = series_per_property(TestSourceKind::Push)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(covered, SeriesStyleProps::NAMES);
        assert!(!SeriesStyleProps::NAMES.is_empty());
    }

    #[test]
    fn a_static_style_property_contributes_no_reactive_source() {
        let series = bare_series();
        let mut versions = Vec::new();
        series.collect_source_versions(&mut versions);

        assert!(versions.is_empty());
        assert!(!series.is_reactive());
        assert!(!series.has_reactive_style_sources());
        assert!(!series.has_temporal_sources());
    }

    #[test]
    fn every_push_style_property_is_seen_by_every_traversal() {
        for (name, series) in series_per_property(TestSourceKind::Push) {
            assert!(series.is_reactive(), "{name} must be reactive");
            assert!(
                series.has_reactive_style_sources(),
                "{name}: has_reactive_style_sources missed the source"
            );

            let mut versions = Vec::new();
            series.collect_source_versions(&mut versions);
            assert_eq!(
                versions.len(),
                1,
                "{name}: collect_source_versions missed the source, so a change \
                 to it would never mark the plot dirty"
            );

            let callback: SharedReactiveCallback = Arc::new(|| {});
            let mut teardowns: Vec<ReactiveTeardown> = Vec::new();
            series.subscribe_push_updates(&callback, &mut teardowns);
            assert_eq!(
                teardowns.len(),
                1,
                "{name}: subscribe_push_updates missed the source"
            );
        }
    }

    #[test]
    fn every_temporal_style_property_is_seen_by_the_temporal_traversal() {
        for (name, series) in series_per_property(TestSourceKind::Temporal) {
            assert!(
                series.has_temporal_sources(),
                "{name}: has_temporal_sources missed the source"
            );
            assert!(series.is_reactive(), "{name} must be reactive");
            // A temporal source is not push-based, so it has no version.
            let mut versions = Vec::new();
            series.collect_source_versions(&mut versions);
            assert!(versions.is_empty(), "{name}: a signal has no version");
        }
    }

    #[test]
    fn a_static_value_and_a_source_are_mutually_exclusive() {
        let mut series = bare_series();

        series.props.alpha.set(Observable::new(0.25_f32).into());
        assert_eq!(
            series.props.alpha.cloned(),
            None,
            "a source retires the static value"
        );
        assert!(series.props.alpha.source().is_some());

        series.props.alpha.set(1.5_f32.into());
        assert_eq!(
            series.props.alpha.cloned(),
            Some(1.0),
            "a static alpha is clamped"
        );
        assert!(
            series.props.alpha.source().is_none(),
            "a static value must retire the source"
        );
    }

    /// The range rule attached to a property at its declaration is applied by
    /// the only path that can write it.
    ///
    /// `PlotSeries` and `SeriesStyle` share one [`SeriesStyleProps`], so they
    /// cannot normalise differently — this pins the ranges themselves.
    #[test]
    fn a_declared_range_is_applied_to_every_static_write() {
        let mut props = SeriesStyleProps::default();

        props.line_width.set((-4.0_f32).into());
        assert_eq!(props.line_width.cloned(), Some(0.1));

        props.marker_size.set(0.0_f32.into());
        assert_eq!(props.marker_size.cloned(), Some(0.1));

        props.alpha.set(1.5_f32.into());
        assert_eq!(props.alpha.cloned(), Some(1.0));

        props.alpha.set((-0.5_f32).into());
        assert_eq!(props.alpha.cloned(), Some(0.0));

        // A property with no declared range is stored verbatim.
        props.color.set(Color::RED.into());
        assert_eq!(props.color.cloned(), Some(Color::RED));
    }

    /// Cloning a property carries its range rule, so a cloned series still
    /// clamps.
    #[test]
    fn cloning_a_property_carries_its_range() {
        let props = SeriesStyleProps::default();
        let mut cloned = props.clone();
        cloned.alpha.set(9.0_f32.into());
        assert_eq!(cloned.alpha.cloned(), Some(1.0));
    }

    /// A live source survives `or_value`; only an unset static value is filled.
    #[test]
    fn a_config_fallback_never_displaces_a_live_source() {
        let mut props = SeriesStyleProps::default();
        props.alpha.set(Observable::new(0.25_f32).into());
        props.alpha.or_value(Some(0.8));

        assert!(props.alpha.source().is_some(), "the source must survive");
        assert!(props.has_reactive_sources());
    }
}
