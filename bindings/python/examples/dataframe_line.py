from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_dataframe, save_example

META = ExampleMeta(
    slug="dataframe-line",
    title="DataFrame input",
    summary="Column selection with pandas-backed `data=` inputs.",
    section="Integration",
    gallery=False,
)


def build_plot():
    frame = sample_dataframe()
    return (
        base_plot("Pandas DataFrame Input")
        .xlabel("time")
        .ylabel("value")
        .line("time", "value", data=frame, label="value", color="#2563eb")
        .line("time", "baseline", data=frame, label="baseline", linestyle="dashed")
        .legend("upper_right")
    )


if __name__ == "__main__":
    save_example(META, build_plot())
