---
title: "Lecture 5: Derivatives"
author: {{author}}
date: {{date_long}}
theme: chalk
---

<!-- slide class="title" -->

# Lecture 5 · Derivatives

Calculus I

**{{author}}** · {{date_long}}

<!-- slide class="agenda" -->

## Today

1. Rates of change
2. The definition of the derivative
3. Rules: power, product, chain
4. A worked example
5. Exercises

<!-- slide -->

## Rates of change

The average rate of change of $f$ between $a$ and $a + h$ is the slope of the secant line:

$$
\frac{f(a + h) - f(a)}{h}
$$

What happens when $h$ gets very small?

<!-- slide -->

## Definition

<div class="definition">

The **derivative** of $f$ at $a$ is

$$
f'(a) = \lim_{h \to 0} \frac{f(a + h) - f(a)}{h}
$$

if this limit exists.

</div>

Geometrically: the slope of the tangent line at $(a, f(a))$.

<!-- slide -->

## The rules

| Rule | Formula |
|:-----|:--------|
| Power | $\dfrac{d}{dx}\,x^n = n\,x^{n-1}$ |
| Product | $(fg)' = f'g + fg'$ |
| Quotient | $\left(\dfrac{f}{g}\right)' = \dfrac{f'g - fg'}{g^2}$ |
| Chain | $(f \circ g)'(x) = f'(g(x))\,g'(x)$ |

<!-- slide -->

## Worked example

Differentiate $f(x) = (3x^2 + 1)^5$.

$$
\begin{aligned}
f'(x) &= 5\,(3x^2 + 1)^4 \cdot \frac{d}{dx}(3x^2 + 1) \\
      &= 5\,(3x^2 + 1)^4 \cdot 6x \\
      &= 30x\,(3x^2 + 1)^4
\end{aligned}
$$

Note: Point out the outer function and the inner function before using the chain rule.

<!-- slide -->

## A function and its derivative

```chart
{
  type: "line",
  data: {
    labels: [-3, -2, -1, 0, 1, 2, 3],
    datasets: [
      { label: "f(x) = x³ − 3x", data: [-18, -2, 2, 0, -2, 2, 18], borderColor: "#facc15", tension: 0.4 },
      { label: "f′(x) = 3x² − 3", data: [24, 9, 0, -3, 0, 9, 24], borderColor: "#86efac", borderDash: [6, 4], tension: 0.4 }
    ]
  },
  options: { plugins: { legend: { labels: { color: "#eef2e2" } } }, scales: { x: { ticks: { color: "#b7c4a8" } }, y: { ticks: { color: "#b7c4a8" } } } }
}
```

Where $f'(x) = 0$, the graph of $f$ is flat: $x = \pm 1$.

<!-- slide -->

## Exercises

1. $\dfrac{d}{dx}\left(4x^3 - 2x + 7\right)$
2. $\dfrac{d}{dx}\left(x^2 \sin x\right)$
3. $\dfrac{d}{dx}\,\sqrt{1 + x^2}$
4. Find the tangent line to $y = x^3$ at $x = 2$.

<!-- slide class="end" -->

# Next week

Applications of derivatives · read chapter 3.4–3.6
