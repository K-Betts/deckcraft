# Curriculum files

A curriculum file tells the Get Ready starter what each year group is taught, so a teacher can say
what they have just taught and get four questions planned for them: **Last lesson** (that
objective), **Last week** (an earlier objective in the unit), **Last topic** (the unit before)
and **Last year** (the year before). When the fourth card's question comes from the unit's prior
learning, or from earlier this year, its heading says so instead: **Prior learning** or **Earlier
this year**.

Load one in **Insert › Get Ready Starter › From what I just taught › Load curriculum file…**. It
is kept with the app's settings on that computer. No curriculum ships with DeckCraft: a
department's scheme of work is its own, and often uses a provider's wording that may not be
shared.

## Format

```json
{
  "version": 1,
  "name": "Our maths 2026-27",
  "courses": [
    {
      "year": "Y7",
      "route": "Core",
      "units": [
        {
          "title": "Fractions",
          "start": "2027-01-04",
          "weeks": 4,
          "objectives": [
            {"text": "Convert between improper fractions and mixed numbers"},
            {"text": "Add and subtract fractions"},
            {"text": "Understanding sin, cos, tan", "search": "identify hypotenuse opposite adjacent sides"}
          ],
          "prior": [{"text": "Converting fractions to decimals"}]
        }
      ]
    }
  ]
}
```

- **courses**: one per year group and route. `year` is `Y7` to `Y11`; `route` is any name
  (`Core`, `Support`, `Higher`, `Foundation`…) or empty when a year has one route.
- **units**, in teaching order: `title`, `start` (the Monday of the first week, `YYYY-MM-DD`) and
  `weeks`. The dialog opens on the unit under way today and guesses the lesson from how far
  through its weeks today is. A unit with no objectives (a holiday, an assessment week) is
  listed but cannot be chosen.
- **objectives**: what is taught, one per lesson or small step, in order.
- **prior** (optional): what pupils should already know; the fourth card practises one of these.
- An objective may have **search**, other words to match when its own wording matches the
  wrong questions, or **skills**, a list of skill keys (as the skill picker stores them, e.g.
  `Y7 Autumn B3 Expressions and equations · S7 Solve 2-step equations`) to use outright.

## How objectives find questions

Each objective is matched to the question bank's skills by its words, with the unit's title
counting for less. Common forms are understood (`factorising`/`factorise`, `pythag`,
`sin, cos, tan`, `position-to-term`). Skills from the class's own year come first, then the years
either side; at Key Stage 4, skills from the class's tier come first.

Check a file before handing it out:

```
deckcraft-cli curriculum check our-curriculum.json
```

It lists every objective with the skills it matched and how well (0 to 1), and marks
`NO MATCH` where nothing fits. Below about 0.45 a match is often poor: give that objective
`search` words. An objective nothing fits gives an empty card the teacher fills in by hand or
with *Change skill…*.

## Making one from a spreadsheet

A department's overview spreadsheet can be converted with a short script. Ringmer's is kept
outside this repository (it uses MathsPad's objective wording) as `school-curriculum/convert_overview.py`
next to the checkout: it reads the year tabs (weeks, unit, objectives, prior learning, objectives
omitted in Support) and the Overview tab's week dates, and writes Core and Support courses for
Years 7 to 9 and Higher and Foundation for Years 10 and 11.
