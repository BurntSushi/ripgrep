// This module is currently a prototype. It was inspired a bit by a similar
// implementation in codesearch[1]. The main difference is that it isn't fixed
// to a particular ngram size and isn't quite as smart about reducing memory
// usage. This was hastily written in order to bootstrap ngram extraction so
// that other things in this crate could be worked on.
//
// [1] - https://github.com/google/codesearch

use std::cmp;
use std::mem;

use bstr::{BString, ByteVec};
use regex_syntax::hir::{self, Hir, HirKind};

#[derive(Clone)]
pub enum GramQuery {
    Literal(BString),
    And(Vec<GramQuery>),
    Or(Vec<GramQuery>),
}

impl GramQuery {
    fn nothing() -> GramQuery {
        GramQuery::Or(vec![])
    }

    fn anything() -> GramQuery {
        GramQuery::And(vec![])
    }

    fn is_nothing(&self) -> bool {
        match *self {
            GramQuery::Or(ref qs) => qs.is_empty(),
            _ => false,
        }
    }

    fn is_anything(&self) -> bool {
        match *self {
            GramQuery::And(ref qs) => qs.is_empty(),
            _ => false,
        }
    }

    fn is_literal(&self) -> bool {
        matches!(*self, GramQuery::Literal(..))
    }

    fn is_and(&self) -> bool {
        matches!(*self, GramQuery::And(..))
    }

    fn is_or(&self) -> bool {
        matches!(*self, GramQuery::Or(..))
    }

    fn unwrap(self) -> GramQuery {
        match self {
            q @ GramQuery::Literal(_) => q,
            GramQuery::And(qs) => GramQuery::and(qs),
            GramQuery::Or(qs) => GramQuery::or(qs),
        }
    }

    fn unwrap_literal(self) -> BString {
        match self {
            GramQuery::Literal(lit) => lit,
            GramQuery::And(_) => panic!("expected literal, but got And"),
            GramQuery::Or(_) => panic!("expected literal, but got Or"),
        }
    }

    fn or(mut queries: Vec<GramQuery>) -> GramQuery {
        queries.retain(|q| !q.is_nothing());

        if queries.iter().any(GramQuery::is_anything) {
            GramQuery::anything()
        } else if queries.len() == 1 {
            queries.pop().unwrap()
        } else if queries.iter().all(GramQuery::is_literal) {
            let set = LiteralSet::from_vec(
                queries.into_iter().map(GramQuery::unwrap_literal).collect(),
            );
            GramQuery::from_set_or(set)
        } else if queries.iter().any(GramQuery::is_or) {
            let mut flat = vec![];
            for q in queries {
                match q {
                    GramQuery::Or(qs) => {
                        flat.extend(qs);
                    }
                    q => flat.push(q),
                }
            }
            GramQuery::or(flat)
        } else {
            GramQuery::Or(queries)
        }
    }

    fn and(mut queries: Vec<GramQuery>) -> GramQuery {
        queries.retain(|q| !q.is_anything());

        if queries.iter().any(GramQuery::is_nothing) {
            GramQuery::nothing()
        } else if queries.len() == 1 {
            queries.pop().unwrap()
        } else if queries.iter().all(GramQuery::is_literal) {
            let set = LiteralSet::from_vec(
                queries.into_iter().map(GramQuery::unwrap_literal).collect(),
            );
            GramQuery::from_set_and(set)
        } else if queries.iter().any(GramQuery::is_and) {
            let mut flat = vec![];
            for q in queries {
                match q {
                    GramQuery::And(qs) => {
                        flat.extend(qs);
                    }
                    q => flat.push(q),
                }
            }
            GramQuery::and(flat)
        } else {
            GramQuery::And(queries)
        }
    }

    fn from_set_or(mut set: LiteralSet) -> GramQuery {
        if set.is_empty() {
            GramQuery::anything()
        } else if set.len() == 1 {
            GramQuery::Literal(set.lits.pop().unwrap())
        } else {
            let lits: Vec<_> =
                set.lits.into_iter().map(GramQuery::Literal).collect();
            GramQuery::Or(lits)
        }
    }

    fn from_set_and(mut set: LiteralSet) -> GramQuery {
        if set.is_empty() {
            GramQuery::anything()
        } else if set.len() == 1 {
            GramQuery::Literal(set.lits.pop().unwrap())
        } else {
            let lits: Vec<_> =
                set.lits.into_iter().map(GramQuery::Literal).collect();
            GramQuery::And(lits)
        }
    }

    fn union(&mut self, q2: GramQuery) {
        use self::GramQuery::*;

        let (or, and) = (GramQuery::or, GramQuery::and);
        let q1 = mem::replace(self, GramQuery::nothing());
        match (q1, q2) {
            (Literal(lit1), Literal(lit2)) => {
                // let set = LiteralSet::from_vec(vec![lit1, lit2]);
                // *self = GramQuery::from_set_or(set);
                *self = or(vec![Literal(lit1), Literal(lit2)]);
            }
            (Literal(lit1), And(qs2)) => {
                *self = or(vec![Literal(lit1), and(qs2)]);
            }
            (Literal(lit1), Or(mut qs2)) => {
                qs2.push(Literal(lit1));
                *self = or(qs2);
            }
            (And(qs1), And(qs2)) => {
                if qs1.iter().all(GramQuery::is_literal)
                    && qs2.iter().all(GramQuery::is_literal)
                {
                    let (common, not1, not2) = factor(qs1, qs2);
                    let mut disjuncts = vec![];
                    if !not1.is_empty() {
                        disjuncts.push(GramQuery::from_set_and(not1));
                    }
                    if !not2.is_empty() {
                        disjuncts.push(GramQuery::from_set_and(not2));
                    }
                    let mut conjuncts = vec![];
                    if !common.is_empty() {
                        conjuncts.push(GramQuery::from_set_and(common));
                    }
                    if !disjuncts.is_empty() {
                        conjuncts.push(or(disjuncts));
                    }
                    *self = and(conjuncts);
                } else {
                    *self = or(vec![and(qs1), and(qs2)]);
                }
            }
            (And(qs1), q2) => {
                *self = or(vec![and(qs1), q2]);
            }
            (Or(mut qs1), q2) => {
                qs1.push(q2);
                *self = or(qs1);
            }
        }
    }

    fn intersect(&mut self, q2: GramQuery) {
        use self::GramQuery::*;

        let (or, and) = (GramQuery::or, GramQuery::and);
        let q1 = mem::replace(self, GramQuery::nothing()).unwrap();
        let q2 = q2.unwrap();
        match (q1, q2) {
            (Literal(lit1), Literal(lit2)) => {
                *self = and(vec![Literal(lit1), Literal(lit2)]);
            }
            (Literal(lit1), And(mut qs2)) => {
                qs2.push(Literal(lit1));
                *self = and(qs2);
            }
            (Literal(lit1), Or(qs2)) => {
                *self = and(vec![Literal(lit1), or(qs2)]);
            }
            (And(mut qs1), q2) => {
                qs1.push(q2);
                *self = and(qs1);
            }
            (Or(qs1), Or(qs2)) => {
                if qs1.iter().all(GramQuery::is_literal)
                    && qs2.iter().all(GramQuery::is_literal)
                {
                    let (common, not1, not2) = factor(qs1, qs2);
                    let mut conjuncts = vec![];
                    if !not1.is_empty() {
                        conjuncts.push(GramQuery::from_set_or(not1));
                    }
                    if !not2.is_empty() {
                        conjuncts.push(GramQuery::from_set_or(not2));
                    }
                    let mut disjuncts = vec![];
                    if !common.is_empty() {
                        disjuncts.push(GramQuery::from_set_or(common));
                    }
                    if !conjuncts.is_empty() {
                        disjuncts.push(and(conjuncts));
                    }
                    *self = or(disjuncts);
                } else {
                    *self = and(vec![or(qs1), or(qs2)]);
                }
            }
            (Or(qs1), q2) => {
                *self = and(vec![or(qs1), q2]);
            }
        }
    }

    fn and_ngrams(&mut self, size: usize, set: &LiteralSet) {
        if set.min_len() < size {
            return;
        }
        let mut qor = GramQuery::nothing();
        for lit in &set.lits {
            if lit.len() < size {
                continue;
            }

            let mut set = LiteralSet::new();
            set.extend(ngrams(size, lit).map(BString::from));
            qor.union(GramQuery::from_set_and(set));
        }
        self.intersect(qor);
    }
}

impl std::fmt::Debug for GramQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            return match *self {
                GramQuery::Literal(ref lit) => {
                    f.debug_tuple("Literal").field(lit).finish()
                }
                GramQuery::And(ref qs) => {
                    f.debug_tuple("And").field(qs).finish()
                }
                GramQuery::Or(ref qs) => {
                    f.debug_tuple("Or").field(qs).finish()
                }
            };
        }

        match *self {
            GramQuery::Literal(ref lit) => {
                let x = format!("{:?}", lit);
                write!(f, "'{}'", &x[1..x.len() - 1])
            }
            GramQuery::And(ref qs) => {
                let it = qs.iter().map(|q| {
                    if q.is_literal() {
                        format!("{:?}", q)
                    } else {
                        format!("({:?})", q)
                    }
                });
                write!(f, "{}", it.collect::<Vec<String>>().join(" & "))
            }
            GramQuery::Or(ref qs) => {
                let it = qs.iter().map(|q| {
                    if q.is_literal() {
                        format!("{:?}", q)
                    } else {
                        format!("({:?})", q)
                    }
                });
                write!(f, "{}", it.collect::<Vec<String>>().join(" | "))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Analysis {
    size: usize,
    query: GramQuery,
    exact: LiteralSet,
    prefix: LiteralSet,
    suffix: LiteralSet,
}

impl Analysis {
    fn exact(size: usize, set: LiteralSet) -> Analysis {
        Analysis {
            size,
            query: GramQuery::anything(),
            exact: set,
            prefix: LiteralSet::new(),
            suffix: LiteralSet::new(),
        }
    }

    fn anything(size: usize) -> Analysis {
        Analysis {
            size,
            query: GramQuery::anything(),
            exact: LiteralSet::new(),
            prefix: LiteralSet::new(),
            suffix: LiteralSet::new(),
        }
    }

    fn exact_one(size: usize, string: BString) -> Analysis {
        Analysis {
            size,
            query: GramQuery::anything(),
            exact: LiteralSet::single(string),
            prefix: LiteralSet::new(),
            suffix: LiteralSet::new(),
        }
    }

    fn empty_string(size: usize) -> Analysis {
        Analysis::exact(size, LiteralSet::single(BString::from("")))
    }

    fn max_len(&self) -> usize {
        cmp::max(
            self.exact.len(),
            cmp::max(self.prefix.len(), self.suffix.len()),
        )
    }

    fn is_exact(&self) -> bool {
        !self.exact.is_empty()
    }

    fn make_inexact(&mut self) {
        if !self.is_exact() {
            return;
        }
        self.prefix = mem::replace(&mut self.exact, LiteralSet::new());
        self.suffix = self.prefix.clone();
    }

    fn save_exact(&mut self) {
        self.query.and_ngrams(self.size, &self.exact);
    }

    fn union(&mut self, mut o: Analysis) {
        if self.is_exact() && o.is_exact() {
            self.exact.union(o.exact);
        } else if self.is_exact() {
            self.save_exact();
            self.make_inexact();
            self.prefix.union(o.prefix);
            self.suffix.union(o.suffix);
        } else if o.is_exact() {
            o.save_exact();
            self.prefix.union(o.exact.clone());
            self.suffix.union(o.exact);
        } else {
            self.prefix.union(o.prefix);
            self.suffix.union(o.suffix);
        }
        self.query.union(o.query);
        self.simplify();
    }

    fn concat(&mut self, mut o: Analysis) {
        let inex_cross = if self.is_exact() && o.is_exact() {
            self.exact.cross(o.exact);
            None
        } else {
            let self_exact = self.is_exact();
            let o_exact = o.is_exact();
            self.make_inexact();
            o.make_inexact();

            let inex_cross = if !self_exact && !o_exact {
                let mut cross = self.suffix.clone();
                cross.cross(o.prefix.clone());
                Some(cross)
            } else {
                None
            };

            if self_exact {
                self.prefix.cross(o.prefix);
            } else if self.prefix.has_empty() {
                self.prefix.union(o.prefix);
            }
            if o_exact {
                self.suffix.cross(o.suffix);
            } else if self.suffix.has_empty() {
                self.suffix.union(o.suffix);
            } else {
                self.suffix = o.suffix;
            }
            inex_cross
        };
        self.query.intersect(o.query);
        if let Some(cross) = inex_cross {
            self.query.and_ngrams(self.size, &cross);
        }
        self.simplify();
    }

    fn simplify(&mut self) {
        if self.is_exact() && self.exact.min_len() >= (self.size + 1) {
            self.save_exact();
            for lit in &self.exact.lits {
                if lit.len() < self.size {
                    self.prefix.lits.push(lit.clone());
                    self.suffix.lits.push(lit.clone());
                } else {
                    self.prefix.lits.push(lit[..self.size - 1].into());
                    self.suffix
                        .lits
                        .push(lit[lit.len() - (self.size - 1)..].into());
                }
            }
            self.exact.clear();
            self.prefix.canonicalize();
            self.suffix.canonicalize();
        }
        if !self.is_exact() {
            self.simplify_prefix();
            self.simplify_suffix();
        }
    }

    fn finalize(&mut self) {
        if self.is_exact() && self.exact.min_len() >= self.size {
            self.save_exact();
            for lit in &self.exact.lits {
                if lit.len() < self.size {
                    self.prefix.lits.push(lit.clone());
                    self.suffix.lits.push(lit.clone());
                } else {
                    self.prefix.lits.push(lit[..self.size - 1].into());
                    self.suffix
                        .lits
                        .push(lit[lit.len() - (self.size - 1)..].into());
                }
            }
            self.exact.clear();
            self.prefix.canonicalize();
            self.suffix.canonicalize();
        }
        if !self.is_exact() {
            self.simplify_prefix();
            self.simplify_suffix();
        }
    }

    fn simplify_prefix(&mut self) {
        self.query.and_ngrams(self.size, &self.prefix);
        self.prefix.retain_prefix(self.size - 1);
    }

    fn simplify_suffix(&mut self) {
        self.query.and_ngrams(self.size, &self.suffix);
        self.suffix.retain_suffix(self.size - 1);
    }
}

#[derive(Clone, Debug)]
struct LiteralSet {
    lits: Vec<BString>,
}

impl LiteralSet {
    fn new() -> LiteralSet {
        LiteralSet { lits: vec![] }
    }

    fn single(lit: BString) -> LiteralSet {
        LiteralSet { lits: vec![lit] }
    }

    fn from_vec(lits: Vec<BString>) -> LiteralSet {
        let mut set = LiteralSet { lits };
        set.canonicalize();
        set
    }

    fn clear(&mut self) {
        self.lits.clear();
    }

    fn retain_prefix(&mut self, max: usize) {
        for lit in &mut self.lits {
            lit.truncate(max);
        }
        self.canonicalize();
    }

    fn retain_suffix(&mut self, max: usize) {
        for lit in &mut self.lits {
            if lit.len() <= max {
                continue;
            }
            let start = lit.len() - max;
            lit.drain(..start);
        }
        self.canonicalize();
    }

    fn extend<I: IntoIterator<Item = BString>>(&mut self, it: I) {
        self.lits.extend(it);
        self.canonicalize();
    }

    fn canonicalize(&mut self) {
        self.lits.sort();
        self.lits.dedup();
    }

    fn factor(self, o: LiteralSet) -> (LiteralSet, LiteralSet, LiteralSet) {
        // TODO: Do this without cloning every literal.

        let (set1, set2) = (self.lits, o.lits);
        let (mut common, mut not1, mut not2) = (vec![], vec![], vec![]);

        let (mut i1, mut i2) = (0, 0);
        while i1 < set1.len() && i2 < set2.len() {
            if set1[i1] < set2[i2] {
                not1.push(set1[i1].clone());
                i1 += 1;
            } else if set2[i2] < set1[i1] {
                not2.push(set2[i2].clone());
                i2 += 1;
            } else {
                common.push(set1[i1].clone());
                i1 += 1;
                i2 += 1;
            }
        }
        while i1 < set1.len() {
            not1.push(set1[i1].clone());
            i1 += 1;
        }
        while i2 < set2.len() {
            not2.push(set2[i2].clone());
            i2 += 1;
        }
        (
            LiteralSet::from_vec(common),
            LiteralSet::from_vec(not1),
            LiteralSet::from_vec(not2),
        )
    }

    fn union(&mut self, o: LiteralSet) {
        self.lits.extend(o.lits);
        self.canonicalize();
    }

    fn cross(&mut self, o: LiteralSet) {
        if o.is_empty() || o.has_only_empty() {
            return;
        }
        if self.is_empty() || self.has_only_empty() {
            *self = o;
            return;
        }

        let orig = mem::take(&mut self.lits);
        for selflit in &orig {
            for olit in &o.lits {
                let mut newlit = selflit.clone();
                newlit.push_str(olit);
                self.lits.push(newlit);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.lits.is_empty()
    }

    fn len(&self) -> usize {
        self.lits.len()
    }

    fn min_len(&self) -> usize {
        self.lits.iter().map(|x| x.len()).min().unwrap_or(0)
    }

    fn has_empty(&self) -> bool {
        self.lits.first().is_some_and(|x| x.is_empty())
    }

    fn has_only_empty(&self) -> bool {
        self.len() == 1 && self.has_empty()
    }
}

#[derive(Clone, Debug)]
pub struct GramQueryBuilder {
    ngram_size: usize,
    limit_len: usize,
    limit_class: usize,
}

impl GramQueryBuilder {
    pub fn new() -> GramQueryBuilder {
        GramQueryBuilder { ngram_size: 3, limit_len: 250, limit_class: 10 }
    }

    pub fn ngram_size(&mut self, size: usize) -> &mut GramQueryBuilder {
        // A size smaller than this doesn't make a ton of sense, particularly
        // given that it is currently measured in bytes, not codepoints (or
        // graphemes).
        assert!(size >= 2);
        self.ngram_size = size;
        self
    }

    pub fn limit_len(&mut self, len: usize) -> &mut GramQueryBuilder {
        self.limit_len = len;
        self
    }

    pub fn limit_class(&mut self, len: usize) -> &mut GramQueryBuilder {
        self.limit_class = len;
        self
    }

    pub fn build(&self, exp: &Hir) -> GramQuery {
        self.build_analysis(exp).query
    }

    fn build_analysis(&self, exp: &Hir) -> Analysis {
        let mut ana = self.b(exp);
        ana.finalize();
        ana
    }

    fn b(&self, exp: &Hir) -> Analysis {
        match *exp.kind() {
            HirKind::Empty | HirKind::Look(_) => {
                Analysis::empty_string(self.ngram_size)
            }
            HirKind::Literal(ref literal) => Analysis::exact_one(
                self.ngram_size,
                BString::from(literal.0.as_ref()),
            ),
            HirKind::Class(hir::Class::Unicode(ref cls)) => {
                if class_over_limit_unicode(cls, self.limit_class) {
                    return Analysis::anything(self.ngram_size);
                }

                let mut set = LiteralSet::new();
                for r in cls.iter() {
                    for cp in (r.start() as u32)..=(r.end() as u32) {
                        let Some(ch) = char::from_u32(cp) else {
                            continue;
                        };
                        set.lits.push(BString::from(ch.to_string()));
                    }
                }
                set.canonicalize();
                Analysis::exact(self.ngram_size, set)
            }
            HirKind::Class(hir::Class::Bytes(ref cls)) => {
                if class_over_limit_bytes(cls, self.limit_class) {
                    return Analysis::anything(self.ngram_size);
                }

                let mut set = LiteralSet::new();
                for r in cls.iter() {
                    for b in r.start()..=r.end() {
                        set.lits.push(BString::from(vec![b]));
                    }
                }
                set.canonicalize();
                Analysis::exact(self.ngram_size, set)
            }
            HirKind::Capture(ref capture) => self.b(&capture.sub),
            HirKind::Repetition(ref rep) => {
                if rep.min == 0 {
                    Analysis::anything(self.ngram_size)
                } else {
                    let mut ana = self.b(&rep.sub);
                    ana.make_inexact();
                    ana
                }
            }
            HirKind::Alternation(ref exps) => {
                let mut ana = self.b(&exps[0]);
                for e in exps.iter().skip(1) {
                    ana.union(self.b(e));
                }
                ana
            }
            HirKind::Concat(ref exps) => {
                let exps = combine_literals(exps);
                let mut ana = Analysis::empty_string(self.ngram_size);
                for e in exps {
                    let next = self.build_literal_or_hir(e);
                    if ana.max_len() + ana.max_len() > self.limit_len {
                        ana.concat(Analysis::anything(self.ngram_size));
                    } else {
                        ana.concat(next);
                    }
                }
                ana
            }
        }
    }

    fn build_literal_or_hir(&self, or: LiteralOrHir) -> Analysis {
        match or {
            LiteralOrHir::Literal(string) => {
                Analysis::exact_one(self.ngram_size, string)
            }
            LiteralOrHir::Other(exp) => self.b(exp),
        }
    }
}

impl Default for GramQueryBuilder {
    fn default() -> GramQueryBuilder {
        GramQueryBuilder::new()
    }
}

fn class_over_limit_unicode(cls: &hir::ClassUnicode, limit: usize) -> bool {
    let mut count = 0;
    for r in cls.iter() {
        if count > limit {
            return true;
        }
        count += (r.end() as u32 - r.start() as u32) as usize;
    }
    count > limit
}

fn class_over_limit_bytes(cls: &hir::ClassBytes, limit: usize) -> bool {
    let mut count = 0;
    for r in cls.iter() {
        if count > limit {
            return true;
        }
        count += (r.end() - r.start()) as usize;
    }
    count > limit
}

#[derive(Debug)]
enum LiteralOrHir<'a> {
    Literal(BString),
    // Guaranteed to never contain a HirKind::Literal.
    Other(&'a Hir),
}

fn combine_literals(concat: &[Hir]) -> Vec<LiteralOrHir<'_>> {
    let mut combined = vec![];
    let mut lit = BString::from(vec![]);
    for exp in concat {
        match *exp.kind() {
            HirKind::Literal(ref literal) => {
                lit.extend_from_slice(&literal.0);
            }
            _ => {
                if !lit.is_empty() {
                    combined.push(LiteralOrHir::Literal(lit));
                    lit = BString::from(vec![]);
                }
                combined.push(LiteralOrHir::Other(exp));
            }
        }
    }
    if !lit.is_empty() {
        combined.push(LiteralOrHir::Literal(lit));
    }
    combined
}

/// Returns all ngrams of the given size in a sliding window fashion over the
/// given literal. If the literal is smaller than the given size, then the
/// entire literal is returned as an ngram. (An empty literal always results in
/// a single empty string returned.)
fn ngrams<'b, B: 'b + AsRef<[u8]> + ?Sized>(
    size: usize,
    lit: &'b B,
) -> impl Iterator<Item = &'b [u8]> {
    let lit = lit.as_ref();
    let size = cmp::min(size, lit.len());
    let end = lit.len() - size;
    (0..=end).map(move |i| &lit[i..i + size])
}

fn factor(
    qs1: Vec<GramQuery>,
    qs2: Vec<GramQuery>,
) -> (LiteralSet, LiteralSet, LiteralSet) {
    let (mut lits1, mut lits2) = (vec![], vec![]);
    for q in qs1 {
        lits1.push(q.unwrap_literal());
    }
    for q in qs2 {
        lits2.push(q.unwrap_literal());
    }

    let set1 = LiteralSet::from_vec(lits1);
    let set2 = LiteralSet::from_vec(lits2);
    set1.factor(set2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex_syntax::ParserBuilder;

    fn parse(pattern: &str) -> Hir {
        ParserBuilder::new().utf8(false).build().parse(pattern).unwrap()
    }

    fn gramq(pattern: &str) -> GramQuery {
        let re = parse(pattern);
        let mut b = GramQueryBuilder::new();
        b.ngram_size(3);
        b.build(&re)
    }

    fn sgramq(pattern: &str) -> String {
        format!("{:?}", gramq(pattern))
    }

    #[test]
    fn iter_ngrams() {
        let get = |size, lit| -> Vec<BString> {
            ngrams(size, lit).map(BString::from).collect()
        };

        assert_eq!(get(3, "foobar"), vec!["foo", "oob", "oba", "bar"]);
        assert_eq!(get(3, "fooba"), vec!["foo", "oob", "oba"]);
        assert_eq!(get(3, "foob"), vec!["foo", "oob"]);
        assert_eq!(get(3, "foo"), vec!["foo"]);
        assert_eq!(get(3, "fo"), vec!["fo"]);
        assert_eq!(get(3, "f"), vec!["f"]);
        assert_eq!(get(3, ""), vec![""]);

        assert_eq!(get(1, "ab"), vec!["a", "b"]);
        assert_eq!(get(1, "a"), vec!["a"]);
        assert_eq!(get(1, ""), vec![""]);

        assert_eq!(get(0, "ab"), vec!["", "", ""]);
        assert_eq!(get(0, "a"), vec!["", ""]);
        assert_eq!(get(0, ""), vec![""]);
    }

    // These tests were taken from codesearch's test suite:
    // https://github.com/google/codesearch
    #[test]
    fn queries() {
        macro_rules! t {
            ($regex:expr, $expected:expr) => {
                assert_eq!(sgramq($regex), $expected);
            };
        }

        t!("foo", "'foo'");
        t!("foob", "'foo' & 'oob'");
        t!("foobar", "'bar' & 'foo' & 'oba' & 'oob'");
        t!("(abc)(def)", "'abc' & 'bcd' & 'cde' & 'def'");
        t!("abc.*def", "'abc' & 'def'");
        t!("abc.*(def|ghi)", "'abc' & ('def' | 'ghi')");
        t!(
            "abc(def|ghi)",
            "'abc' & (('bcd' & 'cde' & 'def') | ('bcg' & 'cgh' & 'ghi'))"
        );
        t!("a+hello", "'ahe' & 'ell' & 'hel' & 'llo'");
        t!(
            "(a+hello|b+world)",
            "('ahe' & 'ell' & 'hel' & 'llo') | ('bwo' & 'orl' & 'rld' & 'wor')"
        );
        t!("a*bbb", "'bbb'");
        t!("a?bbb", "'bbb'");
        t!("(bbb)a*", "'bbb'");
        t!("(bbb)a?", "'bbb'");
        t!("^abc", "'abc'");
        t!("abc$", "'abc'");
        t!("ab[cde]f", "('abc' & 'bcf') | ('abd' & 'bdf') | ('abe' & 'bef')");
        t!("(abc|bac)de", "'cde' & (('abc' & 'bcd') | ('acd' & 'bac'))");

        t!("ab[^cde]f", "");
        t!("ab.f", "");
        t!(".", "");
        t!("(?s).", "");
        t!("()", "");

        // Tests that we keeps track of multiple possible prefix/suffixes.
        t!(
            "[ab][cd][ef]",
            "'ace' | 'acf' | 'ade' | 'adf' | 'bce' | 'bcf' | 'bde' | 'bdf'"
        );
        t!("ab[cd]e", "('abc' & 'bce') | ('abd' & 'bde')");

        // Tests that different sized suffixes works.
        t!("(a|ab)cde", "'cde' & ('acd' | ('abc' & 'bcd'))");
        t!("(a|b|c|d)(ef|g|hi|j)", "");

        // Case insensitivity sanity check.
        t!("(?i)a~~", "'A~~' | 'a~~'");
        t!("(?i)ab~", "'AB~' | 'Ab~' | 'aB~' | 'ab~'");
        t!(
            "(?i)abc",
            "'ABC' | 'ABc' | 'AbC' | 'Abc' | 'aBC' | 'aBc' | 'abC' | 'abc'"
        );

        // Word boundaries.
        t!(r"\b", "");
        t!(r"\B", "");
        t!(r"\babc", "'abc'");
        t!(r"\Babc", "'abc'");
        t!(r"abc\b", "'abc'");
        t!(r"abc\B", "'abc'");
        t!(r"ab\bc", "'abc'");
        t!(r"ab\Bc", "'abc'");

        // Test that factoring out common ngrams works.
        t!("abc|abc", "'abc'");
        t!("(ab|ab)c", "'abc'");
        t!("ab(cab|cat)", "'abc' & 'bca' & ('cab' | 'cat')");
        t!("(z*(abc|def)z*)(z*(abc|def)z*)", "'abc' | 'def'");
        t!("(z*abcz*defz*)|(z*abcz*defz*)", "'abc' & 'def'");
        t!(
            "(z*abcz*defz*(ghi|jkl)z*)|(z*abcz*defz*(mno|prs)z*)",
            // "('abc' & 'def' & ('ghi' | 'jkl' | 'mno' | 'prs'))"
            "('abc' & 'def' & ('ghi' | 'jkl')) \
             | ('abc' & 'def' & ('mno' | 'prs'))"
        );
    }

    #[test]
    fn scratch() {
        // println!("{:#?}", analysis(r"a|b|c"));
        // println!("{:#?}", analysis(r"[2-6]"));
        // println!("{:#?}", analysis(r"abcQ+def(QQ)+xyz"));
        // println!("{:#?}", analysis(r".abc(XYZ)+"));
        // println!("{:#?}", analysis(r".(a)(yz)"));
        // println!("{:#?}", analysis(r"abc.def.ghi"));
        // println!("{:#?}", analysis(r"ZZZ+(foo|bar|baz)(a|b)"));
        // println!("{:#?}", analysis(r"aND|caN|Ha[DS]|WaS"));
        // println!("{:#?}", analysis(r"\|[^|][^|]*\|"));
        // println!("{:#?}", analysis(r"a[act]ggtaaa|tttacc[agt]t"));
        // println!("{:#?}", analysis(r">[^\n]*\n|\n"));
        // println!("{:#?}", analysis(r"abcd|(efgh|(mnop|uvwx|yzab)|qrst)|ijkl"));
        // println!("{:#?}", analysis(r"foo|bar"));
        // println!("{:#?}", analysis(r"foo"));
        // println!("{:#?}", analysis(r"abc.*"));
        // println!("{:#?}", analysis(r"abc.*(def|ghi)"));
        // println!("{:#?}", analysis(r"abc.*def"));
        // println!("{:#?}", analysis(r"abcd.*efgh"));
        // println!("{:#?}", analysis(r"abc(def|ghi)"));
        // println!("{:#?}", analysis(r"a+hello"));
        // println!("{:#?}", analysis(r"(a|b|c|d)(ef|g|hi|j)"));
        // println!("{:#?}", analysis(r"(z*(abc|def)z*)(z*(abc|def))"));
    }
}
