use crate::model::{WacSegment, WacValue};
use bamcensus_core::{
    model::identifier::{Geoid, GeoidType},
    ops::agg::NumericAggregation,
};
use itertools::Itertools;
use kdam::BarExt;
use std::collections::HashMap;

/// groups rows to the target Geoid hierarchy level and then
/// applies the provided aggregation function to the grouped WacValues.
///
/// # Example
///
/// ```rust
/// use bamcensus_core::model::identifier::{Geoid, GeoidType, fips};
/// use bamcensus_core::ops::agg::NumericAggregation;
/// use bamcensus_lehd::model::{WacSegment, WacValue};
/// use bamcensus_lehd::ops::lodes_agg;
///
/// // 2020 populations by county (WAC Segment C000) for two counties in Colorado.
/// let rows = vec![
///   (
///     Geoid::County(fips::State(08), fips::County(213)),
///     vec![WacValue::new(WacSegment::C000, 106497.0)]
///   ),
///   (
///     Geoid::County(fips::State(08), fips::County(215)),
///     vec![WacValue::new(WacSegment::C000, 3858.0)]
///   )
/// ];
/// let target = GeoidType::State;
/// let agg = NumericAggregation::Sum;
/// let result = lodes_agg::aggregate_lodes_wac(&rows, target, agg).unwrap();
/// let expected_cnt = 106497.0 + 3858.0;
/// let expected = vec![
///   (
///     Geoid::State(fips::State(08)),
///     vec![WacValue::new(WacSegment::C000, expected_cnt)]
///   )
/// ];
/// for ((g_a, vs_a), (g_b, vs_b)) in result.into_iter().zip(expected) {
///   assert_eq!(g_a, g_b);
///   for (v_a, v_b) in vs_a.into_iter().zip(vs_b) {
///     assert_eq!(v_a.segment, v_b.segment);
///     assert_eq!(v_a.value, v_b.value);
///   }
/// }
/// ```
pub fn aggregate_lodes_wac(
    rows: &[(Geoid, Vec<WacValue>)],
    target: GeoidType,
    agg: NumericAggregation,
) -> Result<Vec<(Geoid, Vec<WacValue>)>, String> {
    if target == GeoidType::Block {
        // LODES data is stored at the block level, this is a no-op
        return Ok(rows.to_vec());
    }

    // aggregate Geoids
    let (geoid_oks, geoid_errs): (Vec<(Geoid, &Vec<WacValue>)>, Vec<String>) = rows
        .iter()
        .map(|(geoid, values)| {
            let trunc_geoid = geoid.truncate_geoid_to_type(&target)?;
            Ok((trunc_geoid, values))
        })
        .partition_result();

    if !geoid_errs.is_empty() {
        let msg = geoid_errs.into_iter().unique().take(5).join("\n");
        return Err(format!(
            "errors during aggregation. first 5 unique errors: \n{msg}"
        ));
    }

    // nested groupby operation collected into a hashmap
    let mut grouped: HashMap<Geoid, HashMap<WacSegment, Vec<f64>>> = HashMap::new();
    let n_geoid_oks = geoid_oks.len();
    let group_iter_desc = format!("LODES - geoids to {target}");
    let pb1_builder = kdam::BarBuilder::default()
        .total(n_geoid_oks)
        .desc(group_iter_desc);
    let mut pb1 = pb1_builder
        .build()
        .map_err(|e| format!("error building progress bar: {e}"))?;

    for (geoid, values) in geoid_oks.into_iter() {
        for wac in values.iter() {
            match grouped.get_mut(&geoid) {
                Some(inner) => match inner.get_mut(&wac.segment) {
                    Some(inner_vec) => {
                        inner_vec.push(wac.value);
                    }
                    None => {
                        let _ = inner.insert(wac.segment, vec![wac.value]);
                    }
                },
                None => {
                    let mut map = HashMap::new();
                    map.insert(wac.segment, vec![wac.value]);
                    grouped.insert(geoid.clone(), map);
                }
            }
        }
        pb1.update(1)
            .map_err(|e| format!("error updating progress bar: {e}"))?;
    }
    eprintln!();

    // flattended into vector collection
    let n_grouped = grouped.len();
    let reduce_desc = format!("LODES - aggregate by {agg}");
    let pb2_builder = kdam::BarBuilder::default()
        .total(n_grouped)
        .desc(reduce_desc);
    let mut pb2 = pb2_builder
        .build()
        .map_err(|e| format!("error building progress bar: {e}"))?;
    let output: Result<Vec<(Geoid, Vec<WacValue>)>, String> = grouped
        .into_iter()
        .map(|(geoid, map)| {
            let values = map
                .into_iter()
                .map(|(seg, values)| {
                    // let mut mut_values = values;
                    let value = agg.aggregate(&mut values.into_iter());
                    WacValue::new(seg, value)
                })
                .collect_vec();
            pb2.update(1)
                .map_err(|e| format!("error updating progress bar: {e}"))?;
            Ok((geoid, values))
        })
        .collect::<Result<Vec<_>, _>>();
    eprintln!(); // end progress bar

    output
}
