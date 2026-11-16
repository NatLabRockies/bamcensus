from pathlib import Path
import unittest
from unittest.mock import patch

import geopandas as gpd
import shapely
from shapely.geometry import MultiPolygon, Polygon, box

import bamcensus
from bamcensus import _normalize_kwargs, _normalize_polygon


class TestBamcensusPythonAdapter(unittest.TestCase):
    def test_normalize_shapely_polygon_and_multipolygon(self):
        poly = box(-105.0, 39.7, -104.9, 39.8)
        wkt_str = _normalize_polygon(poly)
        self.assertTrue(wkt_str.startswith("POLYGON"))

        mpoly = MultiPolygon([poly, box(-104.8, 39.7, -104.7, 39.8)])
        mwkt_str = _normalize_polygon(mpoly)
        self.assertTrue(mwkt_str.startswith("MULTIPOLYGON"))

    def test_normalize_geodataframe_reprojects_and_unions(self):
        poly1 = box(-105.0, 39.7, -104.95, 39.8)
        poly2 = box(-104.95, 39.7, -104.9, 39.8)
        gdf_4326 = gpd.GeoDataFrame({"id": [1, 2]}, geometry=[poly1, poly2], crs="EPSG:4326")
        gdf_3857 = gdf_4326.to_crs("EPSG:3857")

        wkt_reprojected = _normalize_polygon(gdf_3857)
        parsed = shapely.from_wkt(wkt_reprojected)
        minx, miny, maxx, maxy = parsed.bounds
        self.assertAlmostEqual(minx, -105.0, places=4)
        self.assertAlmostEqual(maxx, -104.9, places=4)
        self.assertAlmostEqual(miny, 39.7, places=4)
        self.assertAlmostEqual(maxy, 39.8, places=4)

    def test_normalize_geojson_dict_and_feature_collection(self):
        poly = box(-105.0, 39.7, -104.9, 39.8)
        geom_dict = shapely.geometry.mapping(poly)
        self.assertTrue(_normalize_polygon(geom_dict).startswith("POLYGON"))

        fc = {
            "type": "FeatureCollection",
            "features": [
                {"type": "Feature", "properties": {}, "geometry": geom_dict},
            ],
        }
        self.assertTrue(_normalize_polygon(fc).startswith("POLYGON"))

    def test_normalize_string_and_pathlike_passthrough(self):
        raw_wkt = "POLYGON ((-105 39, -104 39, -104 40, -105 40, -105 39))"
        self.assertEqual(_normalize_polygon(raw_wkt), raw_wkt)
        p = Path("/tmp/study_area.geojson")
        self.assertEqual(_normalize_polygon(p), "/tmp/study_area.geojson")

    def test_normalize_kwargs_geoids_lists_and_mutual_exclusion(self):
        normalized = _normalize_kwargs(
            {
                "geoids": ["08031", "08059"],
                "acs_query": ["NAME", "B01001_001E"],
                "acs_token": None,
            }
        )
        self.assertEqual(normalized["geoids"], "08031,08059")
        self.assertEqual(normalized["acs_query"], "NAME,B01001_001E")
        self.assertNotIn("acs_token", normalized)

        with self.assertRaises(ValueError):
            _normalize_kwargs(
                {
                    "geoids": "08031",
                    "polygon": box(-105.0, 39.7, -104.9, 39.8),
                }
            )

    def test_resolve_geoids_wrappers(self):
        poly = box(-105.0, 39.7, -104.9, 39.8)
        with patch.object(bamcensus, "_require_api") as mock_api:
            mock_api.return_value.resolve_geoids_python.return_value = [
                "08031000100",
                "08031000200",
            ]
            ids = bamcensus.resolve_geoids(2020, poly, "census_tract")
            self.assertEqual(ids, ["08031000100", "08031000200"])

            mock_api.return_value.resolve_geoids_python.return_value = [
                {"geoid": "08031000100", "geometry": poly.wkt}
            ]
            gdf = bamcensus.resolve_geoids(
                2020, poly, "census_tract", include_geometry=True
            )
            self.assertIsInstance(gdf, gpd.GeoDataFrame)
            self.assertEqual(len(gdf), 1)
            self.assertEqual(gdf.iloc[0]["geoid"], "08031000100")
            self.assertEqual(gdf.crs.to_epsg(), 4326)


if __name__ == "__main__":
    unittest.main()
