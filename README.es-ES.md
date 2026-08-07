

# ruviz

Biblioteca de trazado 2D de alto rendimiento para Rust.

[![Crates.io](https://img.shields.io/crates/v/ruviz)](https://crates.io/crates/ruviz)
[![Documentation](https://docs.rs/ruviz/badge.svg)](https://docs.rs/ruviz)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![CI](https://github.com/Ameyanagi/ruviz/actions/workflows/ci.yml/badge.svg)](https://github.com/Ameyanagi/ruviz/actions/workflows/ci.yml)

## Ejemplos Visuales

Haga clic en cualquier gráfico para abrir su código fuente ejecutable en Rust. Consulte la [galería completa](docs/gallery/README.md)
para ver más tipos de gráficos, temas, diseños para publicación y ejemplos de texto internacional.

| Gráfico de líneas | Gráfico de dispersión | Mapa de calor |
|:---:|:---:|:---:|
| [![Gráfico de líneas de onda sinusoidal](docs/assets/gallery/rust/basic/line_plot.png)](examples/doc_line_plot.rs) | [![Gráfico de dispersión agrupado](docs/assets/gallery/rust/basic/scatter_plot.png)](examples/doc_scatter_plot.rs) | [![Mapa de calor coloreado](docs/assets/gallery/rust/basic/heatmap.png)](examples/doc_heatmap.rs) |

| Gráfico violín | Gráfico radar | Figura de múltiples paneles |
|:---:|:---:|:---:|
| [![Gráfico violín estadístico](docs/assets/gallery/rust/statistical/violin_plot.png)](examples/doc_violin.rs) | [![Gráfico radar multieje](docs/assets/gallery/rust/advanced/radar_chart.png)](examples/doc_radar.rs) | [![Análisis científico multipanels](docs/assets/gallery/rust/publication/scientific_analysis_figure.png)](examples/scientific_showcase.rs) |

## Inicio Rápido

Agregue el crate:

```toml
[dependencies]
ruviz = "0.6.0"
```

Cree y guarde un PNG:

```rust,check
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    Plot::new()
        .line(&x, &y)
        .title("Onda Senoidal")
        .xlabel("x")
        .ylabel("sin(x)")
        .save("sine.png")?;

    Ok(())
}
```

Ejecute con:

```bash
cargo run --release
```

![Example Plot](docs/assets/readme/readme_example.png)

## API Común

La API principal es el constructor fluido `Plot`. Las series se finalizan automáticamente cuando
representa, guarda o inicia otra serie.

```rust,check
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let linear = x.clone();
    let quadratic: Vec<f64> = x.iter().map(|&v| v * v).collect();

    Plot::new()
        .line(&x, &linear)
        .label("Lineal")
        .line(&x, &quadratic)
        .label("Cuadrática")
        .legend(LegendPosition::UpperLeft)
        .theme(Theme::publication())
        .save("series.png")?;

    Ok(())
}
```

Cada gráfico se construye de la misma manera: `Plot::new()`, un método de serie, setters,
`save`. No hay un segundo punto de entrada que aprender:

```rust,check
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    Plot::new()
        .line(&x, &y)
        .title("Línea")
        .save("line.png")?;

    Ok(())
}
```

Las funciones de nivel superior `line`/`scatter`/`bar` y el módulo `ruviz::simple` están en desuso a favor de esa cadena; consulte
[docs/migration/0.6-builder-api.md](docs/migration/0.6-builder-api.md).

## Tipos de Gráficos

El constructor raíz `Plot` expone 29 tipos de gráficos, y esa lista es completa:

- Básicos: line, scatter, bar, histogram, box plot, heatmap
- Distribución: KDE, ECDF, violin, boxen, rug
- Categóricos: strip, swarm, grouped bar, stacked bar
- Composición y polar: pie, estilo donut, radar, polar line
- Continuos, discretos y de error: contour, area, stacked area, fill between, hexbin, step, stem, barras de error simétricas/asimétricas
- Jerárquico: dendrogram
- Vector: quiver
- Auxiliares de diseño: subplots, leyendas, controles de cuadrícula/marcas, anotaciones, insertos

Con la característica `3d`, `Plot3D` añade cuatro más: dispersión 3D, línea 3D, superficie
y wireframe.

Todos, excepto `fill_between`, son métodos de *serie* que siguen la misma estructura:
`Plot::new()`, un método de serie, setters, una llamada terminal; por lo que
`.<series>(..).label(..).color(..).legend_best().save(..)` se compila para todos ellos. `fill_between` es una anotación y no una serie: devuelve
el gráfico en sí, por lo que toma setters de nivel de gráfico (`.title(..)`, `.xlabel(..)`) en lugar
de los de nivel de serie.

Las barras agrupadas, barras apiladas y área apilada toman N columnas de valores con nombre sobre un
eje compartido — `.grouped_bar(&categories, &[("Q1", &q1), ("Q2", &q2)])` — y empujan una serie ordinaria por columna, por lo que cada columna obtiene su propio color de paleta,
su propia entrada en la leyenda y las mismas reglas `.color()`/`.label()` que una línea.

Los gráficos conjuntos y los gráficos de pares son *figuras*, no series: `plots::composite::{jointplot,
pairplot}` devuelven un `SubplotFigure`, el mismo tipo que devuelve `subplots`, por lo que se componen con `.suptitle(..).save(..)` en lugar de con la cadena de series.

Cualquier cosa que no esté en esa lista no tiene método de constructor y no se puede dibujar con esa
cadena, aunque el árbol de código fuente contenga implementaciones de ella. Específicamente,
**KDE 2D, gráfico de regresión y gráfico de residuos no tienen un constructor `Plot`**, y los diagramas
de Sankey y los streamplots no están implementados en absoluto.
La [documentación del módulo ruviz::plots](https://docs.rs/ruviz/latest/ruviz/plots/) enumera
cuáles son cuáles, y una prueba mantiene esa lista sincronizada con el constructor.

## Exportación

- `save("plot.png")` escribe archivos PNG en destinos nativos.
- `render()` devuelve una `Image` en memoria.
- `render_png_bytes()` devuelve bytes PNG.
- `export_svg("plot.svg")` escribe archivos SVG en destinos nativos.
- `render_to_svg()` devuelve una cadena SVG.
- `save_pdf("plot.pdf")` está disponible con la característica `pdf`.

Para destinos de navegador/wasm, utilice auxiliares en memoria como `render_png_bytes()`,
`render_to_svg()` y `Image::encode_png()` en lugar de los auxiliares de exportación con ruta de archivo nativa.

## Características (Features)

Las características predeterminadas son `ndarray_support` y `parallel`.

| Característica | Descripción |
|---------|-------------|
| `3d` | `Plot3D`: dispersión 3D, línea, superficie y wireframe |
| `ndarray_support` | soporte de datos ndarray (canónico) |
| `ndarray` | alias de compatibilidad para `ndarray_support` |
| `polars_support` | soporte de datos polars (canónico) |
| `polars` | alias de compatibilidad para `polars_support` |
| `nalgebra_support` | soporte de datos nalgebra (canónico) |
| `nalgebra` | alias de compatibilidad para `nalgebra_support` |
| `parallel` | rasterización de baldosas en hilos múltiples para el backend 3D por software |
| `simd` | soporte SIMD utilizado por rutas orientadas al rendimiento |
| `performance` | abreviatura para `parallel` + `simd` |
| `gpu` | habilita tipos GPU y metadatos `.gpu(true)` |
| `interactive` | soporte de ventana interactiva independiente (canónico) |
| `window` | alias de compatibilidad para `interactive` |
| `interactive-gpu` | `interactive` + `gpu` |
| `serde` | serializar tipos de temas/configuración |
| `pdf` | exportación PDF mediante SVG-a-PDF |
| `typst-math` | renderizado de texto respaldado por Typst |
| `animation` | soporte de grabación GIF |
| `animation-video` | alias de compatibilidad para `animation`; el video AV1 no está disponible actualmente |
| `svg` | no-op, retenido por compatibilidad (ver abajo) |
| `full` | conjunto amplio de características para compilaciones nativas |

La exportación SVG siempre se compila: `render_to_svg()` y `export_svg()` no necesitan
banderas de características, y la característica `svg` no restringe nada.

`parallel` está habilitado por defecto porque el rasterizador 3D por software renderiza sus
baldosas a través de un pool de rayon. Actualmente **no** afecta a la ruta de rasterizado 2D,
y las propias mediciones del crate la sitúan entre 0.94x y 1.05x en cargas de trabajo 2D
— consulte [docs/benchmarks/rust-feature-impact.md](docs/benchmarks/rust-feature-impact.md).
Mida su propia carga de trabajo antes de activar `performance`.

## Notas sobre el Backend

`.backend(...)`, `.auto_optimize()` y `.get_backend_name()` almacenan o informan
metadatos de preferencia del backend. `auto_optimize()` selecciona Skia de manera conservadora en lugar
de anunciar un backend que no puede ejecutarse en todas las rutas de rasterizado públicas.
Utilice `.resolved_backend_name()` para la ruta PNG nativa de `Plot`, o
`.backend_resolution(...)` para inspeccionar el backend solicitado, el backend real y
cualquier razón explícita de respaldo a Skia para una operación de rasterizado. Las cargas de trabajo de dispersión soportadas se resuelven en DataShader solo cuando ese backend está configurado explícitamente.

Utilice compilaciones `release` y realice un benchmark de su carga de trabajo real antes de agregar características
opcionales de rendimiento. Consulte [Selección de Backend](docs/guide/07_backends.md) y
[Optimización de Rendimiento](docs/guide/08_performance.md).

## Integración con GUI Nativa

Ruviz proporciona adaptadores respaldados por imágenes para los principales frameworks de GUI nativos de Rust.
Cada adaptador soporta gráficos 2D estáticos e interactivos, y reenvía las superficies de características `3d`,
`gpu` y `3d-gpu` sin seleccionar un shell de aplicación:

| Framework | Crate y guía |
|-----------|-----------------|
| egui | [`ruviz-egui`](adapters/gui/ruviz-egui/README.md) |
| Iced | [`ruviz-iced`](adapters/gui/ruviz-iced/README.md) |
| Slint | [`ruviz-slint`](adapters/gui/ruviz-slint/README.md) |
| GPUI | [`ruviz-gpui`](adapters/gpui/README.md) |

Los adaptadores retienen la última imagen exitosa mientras renderizan solicitudes más nuevas
en segundo plano. La presentación GPU sigue siendo respaldada por imágenes: `3d-gpu` renderiza en
la GPU y lee el cuadro completado de vuelta para que el framework de GUI lo cargue.

## Modo de Texto Typst

Habilite el renderizado de texto respaldado por Typst con:

```toml
[dependencies]
ruviz = { version = "0.6.0", features = ["typst-math"] }
```

Luego llame a `.typst(true)`. La familia configurada se pasa al rasterizado plano,
SVG plano y texto Typst. La consistencia de fuentes con nombre depende de que esa fuente esté
disponible para cada renderizador o visor SVG; de lo contrario, puede producirse un respaldo o sustitución específico del backend. Typst resuelve `serif`, `sans-serif` y `monospace`
a familias concretas disponibles. Dado que Typst no tiene un selector genérico cursivo o de fantasía,
esos dos valores utilizan su respaldo sans-serif seleccionado:

```rust,check,features=typst-math
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| (-v).exp()).collect();

    Plot::new()
        .line(&x, &y)
        .title("$f(x) = e^(-x)$")
        .xlabel("$x$")
        .ylabel("$f(x)$")
        .font_family("New Computer Modern Sans")
        .typst(true)
        .save("typst_plot.png")?;

    Ok(())
}
```

Sin `typst-math`, `.typst(true)` y `TextEngineMode::Typst` no se
compilan. Si Typst es opcional en su crate, reenvíe y proteja su propia característica:

```toml
[dependencies]
ruviz = { version = "0.6.0", default-features = false }

[features]
default = []
typst-math = ["ruviz/typst-math"]
```

```rust,check
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| (-v).exp()).collect();

    let mut plot = Plot::new()
        .line(&x, &y)
        .title("$f(x) = e^(-x)$");

    #[cfg(feature = "typst-math")]
    {
        plot = plot.typst(true);
    }

    plot.save("typst_plot.png")?;
    Ok(())
}
```

## Ejemplos

Los ejemplos de documentación de Rust están en `examples/doc_*.rs`.

```bash
cargo run --example doc_line_plot
cargo run --example doc_scatter_plot
cargo run --example doc_typst_text --features typst-math
```

Los ejemplos interactivos requieren la característica `interactive`:

```bash
cargo run --features interactive --example basic_interaction
cargo run --features interactive --example interactive_multi_series
```

Los ejemplos de animación requieren la característica `animation`:

```bash
cargo run --features animation --example animation_basic
cargo run --features animation --example animation_wave
```

## Documentación

- [Inicio Rápido](docs/QUICKSTART.md)
- [Guía de Usuario](docs/guide/README.md)
- [Documentación de la API](https://docs.rs/ruviz)
- [Galería](docs/gallery/README.md)
- [Adaptadores de GUI Nativa](adapters/gui/README.md)
- [Estructura del Repositorio](docs/REPOSITORY_STRUCTURE.md)

## Desarrollo

```bash
cargo test
cargo test --doc
cargo run --example basic_example --release
```

El workspace también contiene crates complementarios y bindings, pero este README
se centra en el crate raíz de Rust. Consulte los READMEs de los subdirectorios para esas superficies de paquete.

## Licencia

Licenciado bajo cualquiera de:

- Licencia Apache, Versión 2.0 ([LICENSE](LICENSE) o http://www.apache.org/licenses/LICENSE-2.0)
- Licencia MIT ([LICENSE](LICENSE) o http://opensource.org/licenses/MIT)

a su elección.
