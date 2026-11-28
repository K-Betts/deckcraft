"""Sorts the bank's questions into the department's strands and skills (bank_taxonomy.json).

`categorise(topic, skill)` returns a skill id such as "A05" from the question's own topic and
skill wording, by the first matching rule below (more specific rules come first). A rule may be
limited to some topics. `None` means no rule fits: the page shows those under "Not sorted yet" and
an editor can place them.  Run `python3 bank_categories.py CATALOG.json` to see what is unsorted.
"""
import json
import os
import sys

STRAND = {"Number": "01_Number", "Algebra": "02_Algebra", "Ratio": "03_Ratio_Proportion_Rates",
          "Geometry": "04_Geometry_Measures", "Probability": "05_Probability", "Statistics": "06_Statistics"}

# (skill id, phrases that appear in the skill's name, topics it applies to or None for any)
RULES = [
    ("A06", ["algebraic fraction"], None),
    # --- Probability and statistics first: their words (Venn, frequency) also appear elsewhere
    ("P11", ["conditional"], None),
    ("P09", ["independent events"], None),
    ("P10", ["dependent events"], None),
    ("P09", ["tree diagram"], None),
    ("P06", ["frequency tree"], None),
    ("P08", ["set notation", "complement of a set", "union of sets", "intersection of sets", "identify and represent sets"], ["Probability"]),
    ("P07", ["venn"], ["Probability"]),
    ("P05", ["sample space", "list outcomes", "list and count outcomes", "from sample space"], None),
    ("P04", ["expected outcomes", "expected frequency"], None),
    ("P03", ["relative frequency", "probability experiments"], None),
    ("P02", ["probability of a single event", "sum to 1", "use diagrams to work out probabilities", "two-way tables", "probabilities from"], ["Probability"]),
    ("P01", ["probability vocabulary", "probability scale"], None),
    ("S25", ["capture"], None),
    ("S24", ["sampling", "bias"], None),
    ("S22", ["questionnaire"], None),
    ("S21", ["types of data"], None),
    ("S20", ["compare distributions", "use data to compare"], None),
    ("S19", ["box plot"], None),
    ("S18", ["cumulative frequency"], None),
    ("S17", ["frequency polygon"], None),
    ("S16", ["histogram"], None),
    ("S15", ["stem"], None),
    ("S14", ["scatter", "correlation", "lines of best fit", "interpolation", "outliers", "non-linear relationships"], ["Statistics"]),
    ("S13", ["line graphs", "time series"], ["Statistics"]),
    ("S12", ["pie chart"], None),
    ("S11", ["bar chart", "vertical line and bar", "vertical line chart"], ["Statistics"]),
    ("S10", ["pictogram", "tally"], None),
    ("S09", ["grouped discrete", "continuous data grouped"], ["Statistics"]),
    ("S04", ["ungrouped frequency table"], ["Statistics"]),
    ("S05", ["grouped frequency table"], ["Statistics"]),
    ("S08", ["frequency table", "two-way table"], ["Statistics"]),
    ("S07", ["most appropriate average", "most appropriate graph", "misleading"], ["Statistics"]),
    ("S06", ["from diagram"], ["Statistics"]),
    ("S03", ["median", "mode"], ["Statistics"]),
    ("S02", ["mean"], ["Statistics"]),
    ("S01", ["range", "averages and range"], ["Statistics"]),
    ("S23", ["collect", "present data", "coordinates in the first quadrant"], ["Statistics"]),
    # --- Ratio, proportion and rates
    ("R08", ["proportion equation", "complex direct proportion"], None),
    ("R08", ["inverse proportion", "direct and inverse proportion graphs", "direct proportion graphs", "conversion graphs", "direct proportion and conversion"], None),
    ("R07", ["direct proportion", "best buy"], None),
    ("R04", ["density", "pressure"], None),
    ("R03", ["speed", "velocity", "acceleration", "distance-time", "distance and time", "rates of change", "flow problems", "compound units"], None),
    ("R01", ["calendar", "timetable", "hours and minutes", "hours, days", "time and"], ["Ratio"]),
    ("R02", ["metric units", "exchange rate", "currenc", "interpret maps", "units"], ["Ratio"]),
    ("R09", ["percentage", "interest", "repeated percentage", "multiplier", "express one number as a fraction or", "express a change"], ["Ratio"]),
    ("R06", ["ratio problems", "ratio and algebra", "solve problems with ratio", "similar shapes"], ["Ratio"]),
    ("R05", ["ratio", "scale diagram", "ratios and scales"], ["Ratio"]),
    # --- Geometry
    ("G25", ["bounds", "compound measure"], ["Geometry"]),
    ("G01", ["name 2-d", "recognise and name polygons", "special quadrilaterals", "parallel and perpendicular lines"], ["Geometry"]),
    ("G24", ["proof", "prove", "arguments"], ["Geometry"]),
    ("G21", ["bearing", "compass points"], ["Geometry"]),
    ("G20", ["scale drawing"], ["Geometry"]),
    ("G19", ["construct", "loci", "locus", "accurately draw", "geometrical equipment"], ["Geometry"]),
    ("G18", ["vector"], ["Geometry"]),
    ("G17", ["reflect", "rotation", "rotate", "translation", "enlargement", "transformation", "invariant", "symmetry"], ["Geometry"]),
    ("G16", ["plans and elevations", "isometric", "nets", "3-d represent"], ["Geometry"]),
    ("G22", ["convert metric units"], ["Geometry"]),
    ("G15", ["volume"], ["Geometry"]),
    ("G14", ["surface area"], ["Geometry"]),
    ("G13", ["congru", "similar", "similarity", "recognise enlargement"], ["Geometry"]),
    ("G12", ["sine rule", "cosine rule", "sine and cosine rule", "non-right", "1/2 ab sin", "ab sin c"], ["Geometry"]),
    ("G11", ["trigonometric graph"], ["Geometry"]),
    ("G10", ["trigonom", "sine", "cosine", "tangent ratio", "hypotenuse, opposite", "opposite and adjacent", "exact trigonometric", "right-angled triangles"], ["Geometry"]),
    ("G09", ["pythagoras", "right-angled", "identify the hypotenuse"], ["Geometry"]),
    ("G08", ["circle theorem", "tangent to a circle"], ["Geometry"]),
    ("A08", ["solve equations with squares and square roots"], ["Geometry"]),
    ("G07", ["circle", "arc", "sector", "pi as a ratio", "circumference"], ["Geometry"]),
    ("G22", ["convert metric units", "units of density", "units"], ["Geometry"]),
    ("G06", ["area of", "perimeter of", "area and perimeter", "perimeter and area", "with perimeter"], ["Geometry"]),
    ("G05", ["parallel", "polygon", "alternate", "co-interior", "exterior angle", "interior angle"], ["Geometry"]),
    ("G04", ["triangle", "quadrilateral"], ["Geometry"]),
    ("G03", ["angle", "notation"], ["Geometry"]),
    ("G06", ["area", "perimeter"], ["Geometry"]),
    ("G02", ["3-d", "3d", "prism", "solid"], ["Geometry"]),
    ("G01", ["2-d", "polygons", "shapes", "special quadrilaterals", "parallel and perpendicular"], ["Geometry"]),
    ("G23", ["coordinate", "midpoint"], ["Geometry"]),
    # --- Algebra
    ("A25", ["proof", "counterexample", "arguments"], ["Algebra"]),
    ("A24", ["gradient of a curve", "area under a curve", "differentiat", "turning point"], ["Algebra"]),
    ("A23", ["velocity", "kinematic", "acceleration"], ["Algebra"]),
    ("A22", ["real-life", "real life", "gradients and intercepts of real"], ["Algebra"]),
    ("A20", ["transformations of graphs", "graph transformation"], ["Algebra"]),
    ("A19", ["trigonometric graph"], ["Algebra"]),
    ("A21", ["equation of a circle", "tangent to a circle", "equations of perpendicular", "advanced"], ["Algebra"]),
    ("A18", ["cubic", "reciprocal", "exponential", "recognise and sketch"], ["Algebra"]),
    ("A17", ["quadratic graph", "intercepts and roots", "quadratic graphs of", "interpret quadratic graphs", "draw more complex quadratic graphs"], ["Algebra"]),
    ("A16", ["straight line", "y = mx", "y = x", "gradient", "intercept", "lines parallel", "lines with a negative", "equation of a line", "rearrange equations to the form", "line y", "link y"], ["Algebra"]),
    ("A15", ["coordinate", "midpoint", "table of values"], ["Algebra"]),
    ("A14", ["function", "inverse functions", "composite"], ["Algebra"]),
    ("A13", ["iteration"], ["Algebra"]),
    ("A12", ["sequence", "nth term", "term-to-term", "find the next term"], ["Algebra"]),
    ("A11", ["simultaneous"], ["Algebra"]),
    ("A10", ["quadratic equation", "complete the square", "quadratic formula", "problems with quadratics", "quadratic inequalit", "solve quadratic"], ["Algebra"]),
    ("A09", ["inequalit"], ["Algebra"]),
    ("A08", ["equation", "solve 1-step", "solve 2-step", "manipulating equations", "approximate solutions", "find solutions"], ["Algebra"]),
    ("A07", ["change the subject", "formula", "formulae", "rearrange"], ["Algebra"]),
    ("A06", ["algebraic fraction", "fractional equations"], ["Algebra"]),
    ("A05", ["factoris", "difference of two squares", "complete the square"], ["Algebra"]),
    ("A04", ["expand", "brackets"], ["Algebra"]),
    ("A03", ["simplif", "collect like", "like and unlike", "indices", "powers", "add and subtract expressions", "laws"], ["Algebra"]),
    ("A02", ["substitut", "use one value", "related facts"], ["Algebra"]),
    ("A01", ["notation", "form algebraic", "represent numbers algebraically", "equality and equivalence", "identities", "identity", "directed number with algebra", "missing terms", "function machine"], ["Algebra"]),
    # --- Number
    ("N05", ["related calculations", "solve multi-step problems"], ["Number"]),
    ("N21", ["calculator", "use of a calculator"], ["Number"]),
    ("N20", ["borrowing", "budget", "insurance", "investment", "jobs and pay", "overseas", "bank account", "value added tax", "ways to pay", "ways to save", "running a house"], ["Number"]),
    ("N19", ["list", "systematic"], ["Number"]),
    ("N18", ["exact answer"], ["Number"]),
    ("N17", ["error interval", "bounds"], ["Number"]),
    ("N16", ["surd", "rationalise", "irrational"], ["Number"]),
    ("N15", ["standard form", "powers of 10", "negative powers of 10", "multiply by 0.1"], ["Number"]),
    ("N14", ["indices", "powers and roots", "higher powers", "square and cube", "square roots", "square, cube", "reciprocal"], ["Number"]),
    ("N13", ["estimat"], ["Number"]),
    ("N12", ["round"], ["Number"]),
    ("N10", ["recurring"], ["Number"]),
    ("N09", ["convert fractions, decimals", "convert simple fractions", "tenths, hundredths", "eighths and thousandths", "fractions, decimals and percentages greater", "represent tenths"], ["Number"]),
    ("N11", ["percentage"], ["Number"]),
    ("N08", ["add and subtract fractions", "add and subtract improper", "add and subtract with fractions", "multiply fractions", "divide fractions", "multiply and divide fractions", "multiply and divide mixed", "multiply a fraction", "divide a fraction", "divide an integer by a fraction", "fraction of an amount", "increase or decrease an amount by a fraction", "use a fraction to find", "solve problems with fractions"], ["Number"]),
    ("N07", ["fraction", "mixed number", "improper"], ["Number"]),
    ("N06", ["factor", "multiple", "prime", "hcf", "lcm", "venn", "highest common", "lowest common"], ["Number"]),
    ("N05", ["order of operations", "use factors to simplify"], ["Number"]),
    ("N04", ["directed number", "zero pairs", "cross zero", "number lines with"], ["Number"]),
    ("N03", ["multiply", "divide", "multiplication", "multiples of 10"], ["Number"]),
    ("N02", ["add and subtract", "subtract", "add "], ["Number"]),
    ("N01", ["place value", "compare and order", "write integers", "decimals on a number line", "intervals on a number line", "integers, real"], ["Number"]),
]


def categorise(topic, skill, keywords=()):
    name = skill.lower()
    for sid, phrases, topics in RULES:
        if topics and topic not in topics:
            continue
        if any(p in name for p in phrases):
            return sid
    return None


def taxonomy():
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "bank_taxonomy.json"), encoding="utf-8") as f:
        return json.load(f)


if __name__ == "__main__":
    items = json.load(open(sys.argv[1], encoding="utf-8"))
    items = items["items"] if isinstance(items, dict) else items
    ids = {k["id"] for s in taxonomy() for k in s["skills"]}
    assert all(r[0] in ids for r in RULES), [r[0] for r in RULES if r[0] not in ids]
    unsorted = {}
    counts = {}
    for it in items:
        sid = categorise(it["topic"], it["skill"])
        counts[sid] = counts.get(sid, 0) + 1
        if sid is None:
            unsorted.setdefault((it["topic"], it["skill"]), 0)
            unsorted[(it["topic"], it["skill"])] += 1
    print(f"{len(items)} questions, {counts.get(None, 0)} not sorted, {len(ids - set(counts))} skills empty: {sorted(ids - set(counts))}")
    for (t, s), n in sorted(unsorted.items()):
        print(f"  {t[:3]} | {s} | {n}")
