---
title: Quarterly business review — {{quarter}}
author: {{author}}
date: {{date_long}}
theme: paper
---

<!-- slide class="title" -->

# Quarterly Business Review

{{quarter}} · Sales & Customer Success

**{{author}}** · {{date_long}}

<!-- slide class="agenda" -->

## Agenda

1. Highlights
2. Revenue and pipeline
3. Wins and misses
4. Customers
5. Priorities for next quarter

<!-- slide -->

## Highlights

<div class="kpis">

- **€6.4 M** bookings, 107 % of target
- **+12 %** revenue vs. last quarter
- **118 %** net revenue retention
- **41** new logos

</div>

Note: The headline: we beat target for the third quarter in a row.

<!-- slide -->

## Revenue

```chart
{
  type: "line",
  data: {
    labels: ["Q1", "Q2", "Q3", "Q4"],
    datasets: [
      { label: "Target (€ M)", data: [4.8, 5.2, 5.6, 6.0], borderColor: "#9ca3af", borderDash: [6, 4] },
      { label: "Actual (€ M)", data: [5.0, 5.5, 5.7, 6.4], borderColor: "#1d4ed8", backgroundColor: "rgba(29,78,216,.12)", fill: true, tension: 0.3 }
    ]
  }
}
```

<!-- slide -->

## Pipeline for next quarter

```chart
{
  type: "bar",
  data: {
    labels: ["Qualified", "Proposal", "Negotiation", "Verbal yes"],
    datasets: [{ label: "Pipeline (€ M)", data: [9.2, 5.1, 3.4, 1.6], backgroundColor: ["#bfdbfe", "#93c5fd", "#3b82f6", "#1d4ed8"] }]
  },
  options: { plugins: { legend: { display: false } } }
}
```

Coverage: **3.1×** next quarter's target.

<!-- slide class="two-col" -->

## Wins and misses

<div class="cols">
<div>

### What went well

- Largest deal ever: €820k with a logistics group
- Partner channel brought 30 % of new logos
- Sales cycle shortened from 74 to 61 days

</div>
<div>

### What didn't

- Lost two renewals to price competition
- Mid-market win rate fell to 21 %
- Onboarding backlog of 3 weeks

</div>
</div>

<!-- slide -->

## Customers

| Segment | Customers | Churn | NPS |
|:--------|----------:|------:|----:|
| Enterprise | 64 | 1.2 % | 52 |
| Mid-market | 310 | 3.8 % | 41 |
| Small business | 1,240 | 6.1 % | 38 |

<!-- slide class="agenda" -->

## Priorities for next quarter

1. Close the €3.4 M in negotiation
2. Fix the onboarding backlog: two extra onboarding managers
3. New mid-market package to win back price-sensitive deals
4. Launch the partner certification programme

<!-- slide class="quote" -->

> We chose them because they listened before they sold.
>
> — CIO of our largest new customer

<!-- slide class="end" -->

# Thank you

Questions and discussion
