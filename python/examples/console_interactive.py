from __future__ import annotations

from _shared import ExampleMeta, base_plot, scatter_series

META = ExampleMeta(
    slug="console-interactive",
    title="Console interactivity",
    summary="Open the native interactive window when running outside Jupyter.",
    section="Interactive workflows",
    gallery=False,
)


def build_plot():
    x, y = scatter_series()
    return (
        base_plot("Native Interactive Window", theme="dark")
        .xlabel("feature")
        .ylabel("response")
        .scatter(x, y)
    )


if __name__ == "__main__":
    build_plot().show()
