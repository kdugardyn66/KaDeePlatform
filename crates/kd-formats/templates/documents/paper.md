---
title: Diffusion on Sparse Networks
author: {{author}}
date: {{month_year}}
theme: classic
numbered: true
---

# Diffusion on Sparse Networks: A Simple Model and Its Limits

<div class="authors">

**{{author}}**<sup>1</sup> · **Jane Doe**<sup>2</sup>

<sup>1</sup> Department of Applied Mathematics, University of Example · <sup>2</sup> Institute for Complex Systems

{{month_year}}

</div>

<div class="abstract">

**Abstract.** We study how information spreads on sparse random networks. Using a mean-field approximation, we derive a closed-form threshold for large outbreaks and compare it with simulations on 10,000 networks. The approximation is accurate when the average degree is above four, but underestimates the threshold on networks with strong clustering. We discuss why, and propose a correction that reduces the error from 23 % to 4 %.

**Keywords:** diffusion, random graphs, epidemic threshold, mean-field theory

</div>

## Introduction

Many processes — rumours, innovations, infections — spread along the links of a network. Whether a small start grows into a large outbreak depends on the structure of the network as much as on the process itself.[^1] For sparse networks, where each node has only a few neighbours, classic results based on well-mixed populations no longer hold.

This paper makes three contributions: a simple derivation of the outbreak threshold, a large simulation study, and a correction for clustered networks.

## Related work

Pastor-Satorras and Vespignani showed that scale-free networks may have no threshold at all [1]. Newman derived exact results for configuration-model networks using generating functions [2]. Our work builds on the latter but keeps the derivation elementary.

## Model

Each node is either *susceptible* or *informed*. An informed node passes the information to each neighbour with probability $p$ and then stops. Let $\langle k \rangle$ and $\langle k^2 \rangle$ be the first two moments of the degree distribution. A large outbreak is possible when

$$
p \, \frac{\langle k^2 \rangle - \langle k \rangle}{\langle k \rangle} > 1 ,
$$

which gives the critical transmission probability

$$
p_c = \frac{\langle k \rangle}{\langle k^2 \rangle - \langle k \rangle}.
$$

For an Erdős–Rényi graph with mean degree $c$, $\langle k^2 \rangle = c^2 + c$, so $p_c = 1/c$.

### Clustering correction

Triangles waste transmissions: a neighbour may already be informed through a shared friend. With global clustering coefficient $C$, we propose

$$
\tilde{p}_c = \frac{p_c}{1 - C\,(1 - p_c)} .
$$

## Results

We simulated the process on 10,000 networks of 5,000 nodes each, for six network families.

| Network family | $\langle k \rangle$ | $C$ | $p_c$ simulated | $p_c$ formula | $\tilde{p}_c$ corrected |
|:---------------|-----:|-----:|-----:|-----:|-----:|
| Erdős–Rényi | 4.0 | 0.00 | 0.251 | 0.250 | 0.250 |
| Erdős–Rényi | 8.0 | 0.00 | 0.126 | 0.125 | 0.125 |
| Configuration (power law) | 6.2 | 0.01 | 0.071 | 0.069 | 0.070 |
| Watts–Strogatz | 6.0 | 0.41 | 0.244 | 0.200 | 0.234 |
| Geometric | 7.5 | 0.58 | 0.231 | 0.153 | 0.222 |

Table: Thresholds from simulation and theory.

```chart
{
  type: "line",
  data: {
    labels: [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
    datasets: [
      { label: "Simulation", data: [0.251, 0.236, 0.233, 0.231, 0.239, 0.244, 0.252], borderColor: "#111", tension: 0.3 },
      { label: "Formula", data: [0.250, 0.214, 0.192, 0.172, 0.161, 0.155, 0.151], borderColor: "#999", borderDash: [6, 4] },
      { label: "Corrected", data: [0.250, 0.236, 0.229, 0.227, 0.233, 0.240, 0.247], borderColor: "#1c3f94" }
    ]
  },
  options: { plugins: { title: { display: true, text: "Threshold vs. clustering coefficient C (⟨k⟩ = 6)" } } }
}
```

## Discussion

The mean-field formula works well for networks with little clustering, which matches earlier results [2]. On clustered networks it underestimates the threshold by up to 34 %. The correction removes most of this error with one extra parameter that is easy to measure.

## Conclusion

A two-line formula predicts whether information can spread on a sparse network. Adding the clustering coefficient makes it reliable for the clustered networks found in practice.

## References

1. R. Pastor-Satorras and A. Vespignani, “Epidemic spreading in scale-free networks,” *Physical Review Letters*, 86(14), 2001.
2. M. E. J. Newman, “Spread of epidemic disease on networks,” *Physical Review E*, 66(1), 2002.
3. D. J. Watts and S. H. Strogatz, “Collective dynamics of ‘small-world’ networks,” *Nature*, 393, 1998.

[^1]: We use “outbreak” for any spreading process, not only diseases.
