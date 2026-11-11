use bamcensus_core::model::identifier::Geoid;
use geo::Geometry;
use itertools::Itertools;
use kdam::BarExt;
use std::collections::HashMap;

type PartitionedJoinResult<T> = (Vec<(Geoid, Geometry, Vec<T>)>, Vec<String>);

/// joins a dataset with a geometry dataset. it is assumed that all Geoids in the data rows
/// are present in the tiger rows. this join builds an index over the geometries, steps through
/// the data row iterator, and looks up the geometry in the index. the geometry value is cloned
/// and added to a tuple with the original data.
pub fn dataset_with_geometries<T>(
    data_rows: Vec<(Geoid, Vec<T>)>,
    tiger_rows: Vec<Vec<(Geoid, Geometry<f64>)>>,
) -> Result<PartitionedJoinResult<T>, String> {
    let mut pb = kdam::Bar::builder()
        .total(data_rows.len())
        .desc("dataset join")
        .build()?;

    let tiger_lookup = tiger_rows
        .into_iter()
        .flatten()
        .collect::<HashMap<Geoid, Geometry>>();

    let (join_dataset, join_errors) = data_rows
        .into_iter()
        .map(|(geoid, lodes_values)| {
            let row = match tiger_lookup.get(&geoid) {
                Some(geometry) => Ok((geoid, geometry.clone(), lodes_values)),
                None => Err(format!(
                    "geometry not found for geoid {}, has {} LODES values from API response",
                    geoid,
                    lodes_values.len()
                )),
            };
            let _ = pb.update(1); // ignore progress failures
            row
        })
        .partition_result();

    eprintln!(); // finish progress bar
    Ok((join_dataset, join_errors))
}
