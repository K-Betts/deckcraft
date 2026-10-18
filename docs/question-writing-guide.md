# Writing question templates (for a person or an AI)

A **template** is a question written once as a recipe. Each time a teacher uses it (and each
Reroll) the app picks new numbers and letters, works the answer out itself, and draws the diagram
to match. This page is the whole format. The block under "Prompt to give an AI" is self-contained:
paste it into a local AI, then add the topics you want.

Check a file before using it:

```
deckcraft-cli bank check my-questions.json
```

It says what is wrong (and in which template) or prints example questions with their answers to
read through. In the app, *Manage Questions ▸ Import questions from a file…* runs the same checks
and shows the problem in words.

## Prompt to give an AI

```
You write question TEMPLATES for a UK secondary maths question bank (KS3 and KS4, National
Curriculum for England). A template is a recipe: the app uses it to make a new version of the
question each time (different numbers and letters), working out the answer automatically.
Reply with JSON only, no commentary.

OUTPUT: one JSON object  {"version": 1, "templates": [ ...templates... ]}

EACH TEMPLATE HAS THESE FIELDS
  "id"         unique, lowercase: topic code, "-t-", 3 digits. Codes: num, alg, rat, geo, sta.
               e.g. "alg-t-014". Continue the numbering I give you.
  "topic"      exactly one of: Number, Algebra, Ratio, Geometry, Statistics
  "subtopic"   a specific skill, e.g. "Solving equations: unknown on both sides"
  "difficulty" 1 = support (needs scaffolding), 2 = core, 3 = stretch, 4 = exam style (GCSE-style
               wording, context and marks, e.g. "(2 marks)"; original questions, never copied)
  "qtype"      fluency (practise the method), reasoning (explain / spot / compare) or problem
               (apply in a new context or several steps)
  "year"       the year group: "Y7", "Y8" … "Y11"
  "source"     where it sits in the scheme of work, e.g. "Y7 Autumn B3 Expressions and equations
               · S7 Solve 2-step equations" (teachers can search it)
  "keywords"   5 to 8 words a teacher might SEARCH for: synonyms and phrases, e.g.
               ["solve","equation","both sides","variable on both sides","balance"]
  "vars"       the numbers/letters that change, as a list of [name, rule] pairs, chosen in order
  "calc"       (optional) values worked out from the vars: [["d","(a - c) * x + b"]]
  "where"      (optional) conditions that must be true: ["a > c", "d != 0"]
  "check"      ALWAYS INCLUDE. Sums that must be true for every question made, written by putting
               the ANSWER BACK into the question. The app tests them on 25 questions and refuses
               the template if one fails, so a wrong formula is caught. Examples:
               "a * x + b == c * x + d"   (the answer x really solves the equation)
               "a * a + b * b == c * c"   (Pythagoras)
               "part1 + part2 == t"       (shares add up to the total)
  "text"       the question pupils see
  "answer"     the answer only (a short working line is fine for "form and solve" questions)
  "diagram"    (optional) a figure, see below

RULES FOR vars (each name is a lowercase word or letter; use exactly ONE rule per variable)
  {"int": [low, high]}                    whole number, inclusive. Either end may be an
                                          expression using EARLIER vars: [1, "m - 1"]
  {"int": [low, high], "not": [0, 1]}     whole number that is not one of those
  {"choose": [5, 10, 25, 50]}             one of these numbers
  {"step": [0.5, 9.5, 0.5]}               low, high, step: 0.5, 1, 1.5 ... (decimals)
  {"pick": ["x","y","n","t"]}             a letter or word; written with [[name]] in text

EXPRESSIONS (in "calc", "where", int ends, and inside [[ ]])
  + - * / % ^   brackets ( )   comparisons == != < <= > >=   && (and)  || (or)
  functions: abs sqrt gcd lcm min max round floor ceil isint sin cos tan asin acos atan
  log10, and sf(x, n) = x rounded to n significant figures
  (trig uses DEGREES). Variables are used by name. NOTHING else is allowed.
  Work the ANSWER FIRST, then derive the rest: choose the answer x, then calculate the number
  that makes it come out. This guarantees whole-number answers.

WRITING NUMBERS INTO TEXT: put an expression in double square brackets: [[a + b]]
  Add a filter after a bar to control how it is written:
    [[a]]              5 or -5 or 0.25  (fractions that are not tidy decimals come out as \frac)
    [[a|coef]]         the number in front of a letter: 1 -> nothing, -1 -> "-", 3 -> "3"
                       "[[a|coef]]x" gives x, -x, 3x
    [[b|signed]]       " + 5" or " - 5" (and nothing for 0): "x[[b|signed]]" gives x + 5
    [[c|sterm:y]]      " + 3y" or " - y" (nothing for 0)
    [[c|term:y]]       "3y" or "-y" or "y" ("0" for 0)
    [[d|sgn]]          "+4" or "-4" (no spaces)
    [[cond|if:Yes/No]] the first word when the condition is true, otherwise the second
    [[n|words]]        4032 -> "four thousand and thirty-two"
    [[n|group]]        82000000 -> "82 000 000" (use for numbers of 5 or more digits)
    [[r|frac]]         \frac{3}{4}          [[r|mixed]]  1\frac{1}{2}
    [[p|dp:1]]         rounded to 1 decimal place, trailing zero kept     [[a|abs]]  positive
    [[v]]              a "pick" variable (a letter or word)
  Maths in text is LaTeX between dollar signs: "Solve $[[a|coef]]x + [[b]] = [[c]]$".
  Remember JSON needs every backslash doubled: \\frac \\times \\sqrt \\pi \\text{ cm}.
  Allowed LaTeX: \\frac{}{} \\sqrt{} ^{} _{} \\times \\div \\pi \\theta \\geq \\leq \\neq \\pm
  \\cdot \\text{}. Write % as plain text outside the dollars ("Find 15% of £80"), never inside.
  Units go inside \\text{}: $12\\text{ cm}^2$.

DIAGRAMS (optional): "diagram": {"kind": ..., ...}. Strings may use [[ ]]; a missing/unknown
value is JSON null (drawn as the "unknown" letter, or "?"). To give a NUMBER (not text) use the
whole string "[[expr|n]]". Kinds and their fields:
  triangle-angles   angles:[3 labels, null=unknown], unknown:"x", sizes:[3 true degrees]
  isosceles-triangle  topAngle:"40°"
  parallel-lines    knownAngle:"115°", unknown:"z"
  right-triangle    base, height, hyp, angle (labels; null = unknown), unknown:"x"
  similar-triangles small:[3 labels], large:[3 labels]  (base, right side, left side)
  angles-line       angles:[labels], sizes:[true degrees], unknown   (angles on a straight line)
  angles-point      same, for angles round a point
  circle            radius or diameter:"5 cm", sector:"60°", centre:"O"
  regular-polygon   sides:5..8, angle:null, at:"exterior" (optional)
  rectangle         width, height     l-shape  labels:[6 sides clockwise from the top]
  trapezium         top, bottom, height     parallelogram  base, height, side
  cuboid            length, width, height
  bar-chart         categories:[...], values:[numbers], yLabel
  number-line       min, max, step, marks:[{"at":2,"open":false,"to":"right"}], dots:[...]
                    a two-sided inequality: marks:[{"at":-2,"open":false,"end":4},{"at":4,"open":true}]
  coordinate-grid   xmin,xmax,ymin,ymax, line:{"m":2,"c":1}, points:[[1,2,"A"]]
                    also lines:[{"m":1,"c":0}|{"x":3}|{"y":-2}, optional "color","label","dash":true],
                    segments:[[x1,y1,x2,y2]], polygons:[{"points":[[x,y],...],"color":"#2F6FDB","label":"P"}]
  venn              a, b, onlyA, both, onlyB, outside (null = unknown), unknown
  tree              first:[["Red","3/5"],...], second:[ [[...],[...]], [[...],[...]] ]
  function-machine  input, steps:["× 3","+ 2"] (null step = hidden), output (null = ?)
  sequence-arrows   terms:[3,7,11,null,19] (null = ?), rule:"+4" (or a list)
  pattern           cells:[[rows, columns], ...] drawn as growing squares
  bar-model         bars:[[{"label":"12","size":12},{"label":null,"size":8}], ...], total:"20"
  table             headers:[...], rows:[[...],...]  (also place-value charts, frequency tables)
  fraction-bar      parts, shaded (one bar) or bars:[{"parts":8,"shaded":3,"label":"A"},...] (same-length bars)
  shade-grid        rows, cols (default 10 x 10), shaded: how many squares are coloured (hundredths, percentages)
  pictogram         key, keyLabel, rows:[{"label","count"}] (symbols = count / key; a half symbol if it does not divide)
  scatter           xmin,xmax,ymin,ymax,xstep,ystep,xlabel,ylabel, points:[[x,y]] (3rd item = point name), line:[[x1,y1],[x2,y2]]
  line-graph        same as scatter, with the points joined in order (time series, distance-time graph)
  polygon-shape     shape: square, rectangle, parallelogram, rhombus, trapezium, kite, equilateral, isosceles, scalene, right-angled
  polygon-angles    angles:[labels], sizes:[true degrees], unknown (a quadrilateral drawn with those angles)
  ruler             length (cm): a line above a ruler
  factor-tree       root: {"v": 60, "l": {...}, "r": {...}}, "?" as a value hides it
  counters          positive, negative (zero-pair and charged-counter questions)
  crossing-lines    theta, labels:[4 sectors anticlockwise from the first line], unknown
  bar-chart         also series:[...] with stacked:true for dual and stacked bars
  number-line       also: labelEvery, hideLabels, blank:[values shown as ?], dots:[{"at":0.3,"label":"A"}]
DIAGRAM RULES (these are shown on a classroom screen, so they must not mislead):
  - Angles are drawn to the sizes in "sizes". ALWAYS give "sizes" for algebraic angle questions
    and make them the real values. Do not let an angle be within 12 degrees of 90 unless a
    right angle is the point (use a "where" such as abs(first - 90) >= 12).
  - Keep each angle at least 25 degrees so labels fit. Keep labels short.
  - The numbers in the diagram, question and answer must all come from the same variables.

JSON MUST BE VALID (the most common mistake): every [ has a matching ], every { a matching }.
  A "vars" entry is  ["v", {"pick": ["x","y"]}]  -- the rule's { } closes, then the entry's ].
  No comments, no trailing commas, straight quotes only. Check the brackets before replying.

QUALITY RULES
  - NEGATIVE NUMBERS: never write "+ [[d]]" where d can be negative (you get "+ -5"). Write
    [[d|signed]] with no sign of your own: "x[[d|signed]]". Same for brackets: "([[b|coef]]x[[c|signed]])".
  - MONEY: always two decimal places: "£[[amount|dp:2]]". Keep percentages out of the dollars.
  - RATIOS must be in simplest form: add "gcd(x, y) == 1" to "where". Sharing questions need the
    total to divide exactly: add "t % (x + y) == 0".
  - MEANS and other averages: choose the numbers so the answer is a whole number (or build the
    data from the answer) and say so in "where" (e.g. "sum % 5 == 0").
  - A LABEL YOU DO NOT WANT drawn must be LEFT OUT of the diagram, not null. null means "the
    unknown to find" and draws a letter. (An area question with a right-angle triangle gives
    base and height only: no "hyp", no "angle".)
  - Do not invent maths: a question and its answer must match exactly. If you write an expand
    question, the answer is the real expansion; test it with a "check".
  - Every answer must be correct for EVERY possible choice of numbers. Check by working
    two or three examples by hand.
  - Choose ranges so answers are sensible for the topic: whole numbers unless the topic needs
    otherwise; avoid answers of 0 and 1 unless intended; avoid questions that look identical
    each time (give variables real ranges, vary the letter with a "pick").
  - Use "where" to rule out degenerate cases (a = c in an equation, zero coefficients, repeated
    numbers, ratios that simplify when they should not).
  - UK spelling, £ and metric units. No real pupils' or schools' names. 1 to 2 sentences.
  - Write 3 to 5 keywords-rich "keywords" per template so a teacher can find it by searching.

EXAMPLE 1 (algebra, no diagram)
{"id":"alg-t-001","topic":"Algebra","subtopic":"Solving equations: unknown on both sides","difficulty":2,
 "keywords":["solve","equation","both sides","variable on both sides","unknown on both sides","balance"],
 "vars":[["a",{"int":[3,9]}],["c",{"int":[1,8]}],["x",{"int":[-9,12],"not":[0]}],
         ["b",{"int":[1,20]}],["v",{"pick":["x","y","n","p"]}]],
 "calc":[["d","(a - c) * x + b"]],
 "where":["a > c","d != 0"],
 "check":["a * x + b == c * x + d"],
 "text":"Solve $[[a|coef]][[v]] + [[b]] = [[c|coef]][[v]][[d|signed]]$",
 "answer":"$[[v]] = [[x]]$"}
(We choose the answer x first, then d is worked out so that ax + b = cx + d really has answer x.)

EXAMPLE 2 (geometry with a true-to-size diagram)
{"id":"geo-t-001","topic":"Geometry","subtopic":"Forming and solving equations: angles on a straight line","difficulty":3,
 "keywords":["angles","straight line","form an equation","solve","algebra","180"],
 "vars":[["p",{"int":[2,4]}],["x",{"int":["ceil(120 / (p + 1))","floor(175 / (p + 1))"]}]],
 "calc":[["q","180 - (p + 1) * x"],["first","p * x"],["second","x + q"]],
 "where":["q >= 5","q <= 60","first >= 25","second >= 25","abs(first - 90) >= 12","abs(second - 90) >= 12"],
 "check":["(p + 1) * x + q == 180"],
 "text":"The angles lie on a straight line. Form and solve an equation to find $x$.",
 "answer":"$[[p+1|coef]]x + [[q]] = 180$, so $x = [[x]]$",
 "diagram":{"kind":"angles-line","angles":["$[[p|coef]]x$°","$x + [[q]]$°"],
            "sizes":["[[first|n]]","[[second|n]]"],"unknown":"x"}}

EXAMPLE 3 (Pythagoras from whole-number triples)
{"id":"geo-t-003","topic":"Geometry","subtopic":"Pythagoras: finding the hypotenuse","difficulty":2,
 "keywords":["pythagoras","right-angled triangle","hypotenuse","length"],
 "vars":[["m",{"int":[2,5]}],["n",{"int":[1,"m - 1"]}],["k",{"int":[1,3]}]],
 "calc":[["a","k * (m * m - n * n)"],["b","k * 2 * m * n"],["c","k * (m * m + n * n)"]],
 "where":["gcd(m, n) == 1","c <= 60","a != b"],
 "check":["a * a + b * b == c * c"],
 "text":"Find the length $x$.",
 "answer":"$x = [[c]]\\text{ cm}$",
 "diagram":{"kind":"right-triangle","base":"[[b]] cm","height":"[[a]] cm","hyp":null,"unknown":"x"}}

NOW WRITE templates for:  <TOPIC / SUBTOPICS / DIFFICULTY / HOW MANY / STARTING ID NUMBER>
Reply with the JSON object only.
```

## After the AI answers

1. In the app: *Insert ▸ Get Ready Starter ▸ Manage questions ▸ Add questions written by an AI*.
   Paste the whole reply (a Markdown fence or a friendly sentence around it is fine) and press
   **Check**. (Or save it as `something.json` and use *Import questions from a file…*, or run
   `deckcraft-cli bank check something.json --samples 5` in a terminal.)
2. If it says something is wrong, it names the place; paste that message back to the AI.
3. If it works, read the example questions and answers it shows (three per recipe). Work two by
   hand. An orange ⚠ is a warning to take seriously (for example "+ −5", or money like £8.4);
   ask the AI to fix it, then check again. When happy press **Add these to my bank**.
4. To share with colleagues, send them the file; they import it the same way.
5. To make questions part of the built-in bank, add them to `assets/questions/bank.json` on a
   branch and open a pull request.

## Fixed questions

A question that never changes can still be written the old way, in a `"questions"` list beside the
templates: `id`, `topic`, `subtopic`, `difficulty` (1 to 3), optional `keywords`, `text`, `answer`,
and optional `needsDiagram: true` with a `diagram` (same kinds as above, no `[[ ]]`).
