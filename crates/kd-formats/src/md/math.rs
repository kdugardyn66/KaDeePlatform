//! LaTeX math as Unicode text, for the native preview: Greek letters, operators, fractions,
//! roots, super- and subscripts, `\text{}`, matrices and `\ce{}` chemistry. Not a typesetter,
//! but readable: `x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` → `x = (−b ± √(b² − 4ac))⁄(2a)`.

const SYMBOLS: &[(&str, &str)] = &[
    ("alpha", "α"),
    ("beta", "β"),
    ("gamma", "γ"),
    ("delta", "δ"),
    ("epsilon", "ε"),
    ("varepsilon", "ε"),
    ("zeta", "ζ"),
    ("eta", "η"),
    ("theta", "θ"),
    ("vartheta", "ϑ"),
    ("iota", "ι"),
    ("kappa", "κ"),
    ("lambda", "λ"),
    ("mu", "μ"),
    ("nu", "ν"),
    ("xi", "ξ"),
    ("pi", "π"),
    ("rho", "ρ"),
    ("sigma", "σ"),
    ("tau", "τ"),
    ("upsilon", "υ"),
    ("phi", "φ"),
    ("varphi", "φ"),
    ("chi", "χ"),
    ("psi", "ψ"),
    ("omega", "ω"),
    ("Gamma", "Γ"),
    ("Delta", "Δ"),
    ("Theta", "Θ"),
    ("Lambda", "Λ"),
    ("Xi", "Ξ"),
    ("Pi", "Π"),
    ("Sigma", "Σ"),
    ("Phi", "Φ"),
    ("Psi", "Ψ"),
    ("Omega", "Ω"),
    ("pm", "±"),
    ("mp", "∓"),
    ("times", "×"),
    ("div", "÷"),
    ("cdot", "·"),
    ("ast", "∗"),
    ("leq", "≤"),
    ("le", "≤"),
    ("geq", "≥"),
    ("ge", "≥"),
    ("neq", "≠"),
    ("ne", "≠"),
    ("approx", "≈"),
    ("equiv", "≡"),
    ("sim", "∼"),
    ("simeq", "≃"),
    ("propto", "∝"),
    ("infty", "∞"),
    ("sum", "∑"),
    ("prod", "∏"),
    ("int", "∫"),
    ("iint", "∬"),
    ("oint", "∮"),
    ("partial", "∂"),
    ("nabla", "∇"),
    ("to", "→"),
    ("rightarrow", "→"),
    ("leftarrow", "←"),
    ("Rightarrow", "⇒"),
    ("Leftarrow", "⇐"),
    ("leftrightarrow", "↔"),
    ("Leftrightarrow", "⇔"),
    ("implies", "⇒"),
    ("iff", "⇔"),
    ("mapsto", "↦"),
    ("in", "∈"),
    ("notin", "∉"),
    ("subset", "⊂"),
    ("subseteq", "⊆"),
    ("supset", "⊃"),
    ("cup", "∪"),
    ("cap", "∩"),
    ("emptyset", "∅"),
    ("forall", "∀"),
    ("exists", "∃"),
    ("neg", "¬"),
    ("land", "∧"),
    ("lor", "∨"),
    ("ldots", "…"),
    ("cdots", "⋯"),
    ("dots", "…"),
    ("vdots", "⋮"),
    ("ddots", "⋱"),
    ("degree", "°"),
    ("circ", "∘"),
    ("angle", "∠"),
    ("perp", "⊥"),
    ("parallel", "∥"),
    ("hbar", "ℏ"),
    ("ell", "ℓ"),
    ("Re", "ℜ"),
    ("Im", "ℑ"),
    ("langle", "⟨"),
    ("rangle", "⟩"),
    ("lfloor", "⌊"),
    ("rfloor", "⌋"),
    ("lceil", "⌈"),
    ("rceil", "⌉"),
    ("quad", "  "),
    ("qquad", "    "),
    ("sin", "sin"),
    ("cos", "cos"),
    ("tan", "tan"),
    ("log", "log"),
    ("ln", "ln"),
    ("exp", "exp"),
    ("lim", "lim"),
    ("max", "max"),
    ("min", "min"),
    ("det", "det"),
    ("ge", "≥"),
];

fn sup(c: char) -> Option<char> {
    Some(match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' | '−' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        'a' => 'ᵃ',
        'b' => 'ᵇ',
        'c' => 'ᶜ',
        'd' => 'ᵈ',
        'e' => 'ᵉ',
        'k' => 'ᵏ',
        'm' => 'ᵐ',
        'o' => 'ᵒ',
        'p' => 'ᵖ',
        't' => 'ᵗ',
        'x' => 'ˣ',
        'y' => 'ʸ',
        'T' => 'ᵀ',
        ' ' => ' ',
        _ => return None,
    })
}

fn sub(c: char) -> Option<char> {
    Some(match c {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '-' | '−' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'k' => 'ₖ',
        'n' => 'ₙ',
        'o' => 'ₒ',
        'x' => 'ₓ',
        'm' => 'ₘ',
        'p' => 'ₚ',
        's' => 'ₛ',
        't' => 'ₜ',
        'r' => 'ᵣ',
        ' ' => ' ',
        _ => return None,
    })
}

/// The group starting at `i` (`{…}` or one character / command): its text and the next index.
fn group(c: &[char], mut i: usize) -> (String, usize) {
    while i < c.len() && c[i] == ' ' {
        i += 1;
    }
    if i >= c.len() {
        return (String::new(), i);
    }
    if c[i] == '{' {
        let mut depth = 0;
        let start = i + 1;
        while i < c.len() {
            match c[i] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return (c[start..i].iter().collect(), i + 1);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        return (c[start..].iter().collect(), c.len());
    }
    if c[i] == '\\' {
        let s = i;
        i += 1;
        while i < c.len() && c[i].is_alphabetic() {
            i += 1;
        }
        if i == s + 1 {
            i += 1;
        }
        return (c[s..i.min(c.len())].iter().collect(), i.min(c.len()));
    }
    (c[i].to_string(), i + 1)
}

fn script(s: &str, f: fn(char) -> Option<char>, mark: char) -> String {
    let t = to_unicode(s);
    match t.chars().map(f).collect::<Option<String>>() {
        Some(u) => u,
        None if t.chars().count() == 1 => format!("{mark}{t}"),
        None => format!("{mark}({t})"),
    }
}

/// Wrap in parentheses when it is more than one term.
fn paren(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() <= 1 || s.chars().all(|c| c.is_ascii_digit()) { s.to_string() } else { format!("({s})") }
}

/// LaTeX to Unicode text.
pub fn to_unicode(tex: &str) -> String {
    let c: Vec<char> = tex.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        match ch {
            '\\' => {
                let s = i + 1;
                let mut j = s;
                while j < c.len() && c[j].is_alphabetic() {
                    j += 1;
                }
                if j == s {
                    // \\ \, \; \{ \}
                    let n = c.get(s).copied().unwrap_or(' ');
                    out.push_str(match n {
                        '\\' => "\n",
                        ',' | ';' | ':' | '!' | ' ' => " ",
                        '{' => "{",
                        '}' => "}",
                        '%' => "%",
                        '$' => "$",
                        '&' => "&",
                        '_' => "_",
                        _ => "",
                    });
                    i = s + 1;
                    continue;
                }
                let name: String = c[s..j].iter().collect();
                i = j;
                match name.as_str() {
                    "frac" | "dfrac" | "tfrac" => {
                        let (a, k) = group(&c, i);
                        let (b, k) = group(&c, k);
                        out.push_str(&format!("{}⁄{}", paren(&to_unicode(&a)), paren(&to_unicode(&b))));
                        i = k;
                    }
                    "sqrt" => {
                        let mut k = i;
                        let mut index = String::new();
                        if c.get(k) == Some(&'[') {
                            let e = c[k..].iter().position(|x| *x == ']').map(|p| k + p).unwrap_or(k);
                            index = c[k + 1..e].iter().collect();
                            k = e + 1;
                        }
                        let (a, k) = group(&c, k);
                        let root = match index.as_str() {
                            "3" => "∛",
                            "4" => "∜",
                            _ => "√",
                        };
                        out.push_str(&format!("{root}{}", paren(&to_unicode(&a))));
                        i = k;
                    }
                    "text" | "mathrm" | "textrm" | "mathbf" | "boldsymbol" | "mathit" | "operatorname" | "textbf" | "mbox" => {
                        let (a, k) = group(&c, i);
                        out.push_str(if name.starts_with("text") || name == "mbox" { &a } else { "" });
                        if !(name.starts_with("text") || name == "mbox") {
                            out.push_str(&to_unicode(&a));
                        }
                        i = k;
                    }
                    "mathbb" => {
                        let (a, k) = group(&c, i);
                        for ch in a.chars() {
                            out.push(match ch {
                                'R' => 'ℝ',
                                'N' => 'ℕ',
                                'Z' => 'ℤ',
                                'Q' => 'ℚ',
                                'C' => 'ℂ',
                                'P' => 'ℙ',
                                o => o,
                            });
                        }
                        i = k;
                    }
                    "ce" => {
                        let (a, k) = group(&c, i);
                        out.push_str(&chem(&a));
                        i = k;
                    }
                    "vec" | "hat" | "bar" | "overline" | "tilde" | "dot" => {
                        let (a, k) = group(&c, i);
                        let mark = match name.as_str() {
                            "vec" => '\u{20D7}',
                            "hat" => '\u{0302}',
                            "tilde" => '\u{0303}',
                            "dot" => '\u{0307}',
                            _ => '\u{0305}',
                        };
                        out.push_str(&to_unicode(&a));
                        out.push(mark);
                        i = k;
                    }
                    "begin" | "end" => {
                        let (_, k) = group(&c, i);
                        i = k;
                    }
                    "left" | "right" | "big" | "Big" | "bigg" | "Bigg" | "displaystyle" | "limits" | "nolimits" => {}
                    other => match SYMBOLS.iter().find(|(n, _)| *n == other) {
                        Some((_, u)) => out.push_str(u),
                        None => out.push_str(other),
                    },
                }
            }
            '^' => {
                let (a, k) = group(&c, i + 1);
                out.push_str(&script(&a, sup, '^'));
                i = k;
            }
            '_' => {
                let (a, k) = group(&c, i + 1);
                out.push_str(&script(&a, sub, '_'));
                i = k;
            }
            '{' | '}' => i += 1,
            '&' => {
                out.push_str("  ");
                i += 1;
            }
            '-' => {
                out.push('−');
                i += 1;
            }
            '~' => {
                out.push(' ');
                i += 1;
            }
            _ => {
                out.push(ch);
                i += 1;
            }
        }
    }
    // tidy spaces around the operators MdCraft's examples use
    let mut tidy = String::with_capacity(out.len());
    let mut prev_space = false;
    for ch in out.chars() {
        if ch == ' ' {
            if !prev_space {
                tidy.push(ch);
            }
            prev_space = true;
        } else {
            tidy.push(ch);
            prev_space = ch == '\n';
        }
    }
    tidy.trim().to_string()
}

/// `\ce{2H2 + O2 -> 2H2O}` → `2H₂ + O₂ → 2H₂O`.
fn chem(s: &str) -> String {
    let s = s.replace("<->", "⇌").replace("<=>", "⇌").replace("->", "→").replace("<-", "←");
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for ch in s.chars() {
        if ch.is_ascii_digit() && prev.is_some_and(|p| p.is_alphabetic() || p == ')') {
            out.push(sub(ch).unwrap_or(ch));
        } else {
            out.push(ch);
        }
        if !(ch.is_ascii_digit() && prev.is_some_and(|p| p.is_alphabetic() || p == ')')) {
            prev = Some(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_unicode;

    #[test]
    fn readable_math() {
        assert_eq!(to_unicode("E = mc^2"), "E = mc²");
        assert_eq!(to_unicode("x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}"), "x = (−b ± √(b² − 4ac))⁄(2a)");
        assert_eq!(to_unicode("\\sum_{i=1}^{n} i"), "∑ᵢ₌₁ⁿ i");
        assert_eq!(to_unicode("\\alpha_0 \\leq \\beta"), "α₀ ≤ β");
        assert_eq!(to_unicode("\\ce{2H2 + O2 -> 2H2O}"), "2H₂ + O₂ → 2H₂O");
        assert_eq!(to_unicode("\\mathbb{R}^n"), "ℝⁿ");
        assert_eq!(to_unicode("\\text{if } x"), "if x");
    }
}
