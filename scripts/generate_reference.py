#!/usr/bin/env python3
"""Generate matplotlib reference images for visual regression testing.

This script generates reference images using matplotlib/seaborn that
serve as visual baselines for comparing ruviz output.

Usage:
    python scripts/generate_reference.py [plot_type]

    # Generate all reference images
    python scripts/generate_reference.py

    # Generate specific plot type
    python scripts/generate_reference.py kde
    python scripts/generate_reference.py ecdf

Output:
    tests/visual/reference/matplotlib/{plot_type}.png
"""

import argparse
import os
import sys
from pathlib import Path

import numpy as np

# Try to import matplotlib and seaborn
try:
    import matplotlib.pyplot as plt
    import matplotlib
    matplotlib.use('Agg')  # Non-interactive backend
except ImportError:
    print("Error: matplotlib not installed. Run: pip install matplotlib")
    sys.exit(1)

try:
    import seaborn as sns
except ImportError:
    print("Warning: seaborn not installed. Some plots may not be available.")
    sns = None

# Configuration
SEED = 42
OUTPUT_DIR = Path(__file__).parent.parent / "tests" / "visual" / "reference" / "matplotlib"
FIGURE_SIZE = (6.4, 4.8)  # matplotlib default
DPI = 100


def ensure_output_dir():
    """Create output directory if it doesn't exist."""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)


def generate_test_data(n=1000, seed=SEED):
    """Generate reproducible test data."""
    np.random.seed(seed)
    return {
        'normal': np.random.randn(n),
        'bimodal': np.concatenate([
            np.random.randn(n // 2) - 2,
            np.random.randn(n // 2) + 2
        ]),
        'uniform': np.random.uniform(-3, 3, n),
        'exponential': np.random.exponential(1, n),
        'xy': (np.linspace(0, 10, n), np.sin(np.linspace(0, 10, n)) + np.random.randn(n) * 0.1),
    }


def generate_kde(data):
    """Generate KDE plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    if sns:
        sns.kdeplot(data['normal'], ax=ax, fill=True, alpha=0.5, label='Normal')
        sns.kdeplot(data['bimodal'], ax=ax, fill=True, alpha=0.5, label='Bimodal')
    else:
        ax.hist(data['normal'], bins=50, density=True, alpha=0.5, label='Normal')
        ax.hist(data['bimodal'], bins=50, density=True, alpha=0.5, label='Bimodal')

    ax.set_title('KDE Plot')
    ax.set_xlabel('Value')
    ax.set_ylabel('Density')
    ax.legend()
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'kde.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'kde.png'}")


def generate_ecdf(data):
    """Generate ECDF plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    if sns:
        sns.ecdfplot(data['normal'], ax=ax, label='Normal')
        sns.ecdfplot(data['bimodal'], ax=ax, label='Bimodal')
    else:
        # Manual ECDF calculation
        for name, d in [('Normal', data['normal']), ('Bimodal', data['bimodal'])]:
            sorted_data = np.sort(d)
            y = np.arange(1, len(sorted_data) + 1) / len(sorted_data)
            ax.step(sorted_data, y, where='post', label=name)

    ax.set_title('ECDF Plot')
    ax.set_xlabel('Value')
    ax.set_ylabel('Cumulative Probability')
    ax.legend()
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'ecdf.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'ecdf.png'}")


def generate_violin(data):
    """Generate violin plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    parts = ax.violinplot([data['normal'], data['bimodal'], data['uniform']],
                          positions=[1, 2, 3], showmeans=True, showmedians=True)

    ax.set_xticks([1, 2, 3])
    ax.set_xticklabels(['Normal', 'Bimodal', 'Uniform'])
    ax.set_title('Violin Plot')
    ax.set_ylabel('Value')
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'violin.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'violin.png'}")


def generate_step(data):
    """Generate step plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    x, y = data['xy']
    x_short, y_short = x[:50], y[:50]  # Use fewer points for visibility

    ax.step(x_short, y_short, where='pre', label='pre')
    ax.step(x_short, y_short + 0.5, where='mid', label='mid')
    ax.step(x_short, y_short + 1.0, where='post', label='post')

    ax.set_title('Step Plot')
    ax.set_xlabel('X')
    ax.set_ylabel('Y')
    ax.legend()
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'step.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'step.png'}")


def generate_contour(data):
    """Generate contour plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    # Generate 2D data
    x = np.linspace(-3, 3, 100)
    y = np.linspace(-3, 3, 100)
    X, Y = np.meshgrid(x, y)
    Z = np.exp(-(X**2 + Y**2) / 2) + 0.5 * np.exp(-((X-1)**2 + (Y-1)**2) / 0.5)

    contour = ax.contourf(X, Y, Z, levels=20, cmap='viridis')
    ax.contour(X, Y, Z, levels=20, colors='white', linewidths=0.5, alpha=0.5)
    fig.colorbar(contour, ax=ax, label='Value')

    ax.set_title('Contour Plot')
    ax.set_xlabel('X')
    ax.set_ylabel('Y')

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'contour.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'contour.png'}")


def generate_hexbin(data):
    """Generate hexbin plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    # Generate 2D scatter data
    np.random.seed(SEED)
    x = np.random.randn(5000)
    y = x + np.random.randn(5000) * 0.5

    hb = ax.hexbin(x, y, gridsize=30, cmap='viridis')
    fig.colorbar(hb, ax=ax, label='Count')

    ax.set_title('Hexbin Plot')
    ax.set_xlabel('X')
    ax.set_ylabel('Y')

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'hexbin.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'hexbin.png'}")


def generate_pie(data):
    """Generate pie chart reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    values = [30, 25, 20, 15, 10]
    labels = ['A', 'B', 'C', 'D', 'E']
    explode = [0.1, 0, 0, 0, 0]

    ax.pie(values, labels=labels, explode=explode, autopct='%1.1f%%',
           shadow=True, startangle=90)
    ax.set_title('Pie Chart')

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'pie.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'pie.png'}")


def generate_errorbar(data):
    """Generate error bar plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    x = np.arange(1, 11)
    y = np.sin(x) + np.random.randn(10) * 0.1
    y_err = np.random.uniform(0.1, 0.3, 10)

    ax.errorbar(x, y, yerr=y_err, fmt='o-', capsize=3, capthick=1, label='Data')

    ax.set_title('Error Bar Plot')
    ax.set_xlabel('X')
    ax.set_ylabel('Y')
    ax.legend()
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'errorbar.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'errorbar.png'}")


def generate_histogram(data):
    """Generate histogram plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    ax.hist(data['normal'], bins=30, alpha=0.7, edgecolor='black',
            linewidth=0.8, label='Normal')
    ax.hist(data['bimodal'], bins=30, alpha=0.7, edgecolor='black',
            linewidth=0.8, label='Bimodal')

    ax.set_title('Histogram')
    ax.set_xlabel('Value')
    ax.set_ylabel('Frequency')
    ax.legend()
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'histogram.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'histogram.png'}")


def generate_boxplot(data):
    """Generate box plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    box_data = [data['normal'], data['bimodal'], data['uniform'], data['exponential']]
    bp = ax.boxplot(box_data, tick_labels=['Normal', 'Bimodal', 'Uniform', 'Exponential'],
                    patch_artist=True)

    # Style the boxes with fill color
    colors = ['#1f77b4', '#ff7f0e', '#2ca02c', '#d62728']
    for patch, color in zip(bp['boxes'], colors):
        patch.set_facecolor(color)
        patch.set_alpha(0.7)

    ax.set_title('Box Plot')
    ax.set_ylabel('Value')
    ax.grid(True, alpha=0.3)

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'boxplot.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'boxplot.png'}")


def generate_heatmap(data):
    """Generate heatmap plot reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI)

    # Generate correlation-like data
    np.random.seed(SEED)
    matrix = np.random.randn(8, 8)
    matrix = (matrix + matrix.T) / 2  # Make symmetric

    if sns:
        sns.heatmap(matrix, ax=ax, cmap='viridis', annot=True, fmt='.2f',
                    linewidths=0.5, linecolor='white')
    else:
        im = ax.imshow(matrix, cmap='viridis')
        fig.colorbar(im, ax=ax)
        # Add text annotations
        for i in range(matrix.shape[0]):
            for j in range(matrix.shape[1]):
                ax.text(j, i, f'{matrix[i, j]:.2f}', ha='center', va='center',
                        color='white' if abs(matrix[i, j]) > 0.5 else 'black',
                        fontsize=8)

    ax.set_title('Heatmap')

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'heatmap.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'heatmap.png'}")


def generate_radar(data):
    """Generate radar (spider) chart reference."""
    fig, ax = plt.subplots(figsize=FIGURE_SIZE, dpi=DPI, subplot_kw=dict(polar=True))

    # Categories
    categories = ['Speed', 'Power', 'Range', 'Defense', 'Health', 'Magic']
    N = len(categories)

    # Values for two series
    values1 = [0.8, 0.6, 0.7, 0.5, 0.9, 0.4]
    values2 = [0.5, 0.9, 0.4, 0.8, 0.6, 0.7]

    # Close the polygon
    angles = np.linspace(0, 2 * np.pi, N, endpoint=False).tolist()
    values1 += values1[:1]
    values2 += values2[:1]
    angles += angles[:1]

    ax.plot(angles, values1, 'o-', linewidth=2, label='Player 1')
    ax.fill(angles, values1, alpha=0.25)
    ax.plot(angles, values2, 'o-', linewidth=2, label='Player 2')
    ax.fill(angles, values2, alpha=0.25)

    ax.set_xticks(angles[:-1])
    ax.set_xticklabels(categories)
    ax.set_title('Radar Chart')
    ax.legend(loc='upper right', bbox_to_anchor=(1.2, 1.0))

    fig.tight_layout()
    fig.savefig(OUTPUT_DIR / 'radar.png')
    plt.close(fig)
    print(f"Generated: {OUTPUT_DIR / 'radar.png'}")


def generate_all():
    """Generate all reference images."""
    print(f"Generating reference images in: {OUTPUT_DIR}")
    ensure_output_dir()

    data = generate_test_data()

    generators = [
        ('kde', generate_kde),
        ('ecdf', generate_ecdf),
        ('violin', generate_violin),
        ('step', generate_step),
        ('contour', generate_contour),
        ('hexbin', generate_hexbin),
        ('pie', generate_pie),
        ('errorbar', generate_errorbar),
        ('histogram', generate_histogram),
        ('boxplot', generate_boxplot),
        ('heatmap', generate_heatmap),
        ('radar', generate_radar),
    ]

    for name, generator in generators:
        try:
            generator(data)
        except Exception as e:
            print(f"Error generating {name}: {e}")

    print(f"\nGenerated {len(generators)} reference images.")


def main():
    parser = argparse.ArgumentParser(description='Generate matplotlib reference images')
    parser.add_argument('plot_type', nargs='?', default='all',
                        help='Plot type to generate (or "all")')
    args = parser.parse_args()

    ensure_output_dir()
    data = generate_test_data()

    generators = {
        'kde': generate_kde,
        'ecdf': generate_ecdf,
        'violin': generate_violin,
        'step': generate_step,
        'contour': generate_contour,
        'hexbin': generate_hexbin,
        'pie': generate_pie,
        'errorbar': generate_errorbar,
        'histogram': generate_histogram,
        'boxplot': generate_boxplot,
        'heatmap': generate_heatmap,
        'radar': generate_radar,
        'all': generate_all,
    }

    if args.plot_type not in generators:
        print(f"Unknown plot type: {args.plot_type}")
        print(f"Available types: {', '.join(generators.keys())}")
        sys.exit(1)

    if args.plot_type == 'all':
        generate_all()
    else:
        generators[args.plot_type](data)


if __name__ == '__main__':
    main()
