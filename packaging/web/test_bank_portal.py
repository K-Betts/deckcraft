"""python3 -m unittest packaging/web/test_bank_portal.py"""
import os, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bank_portal as b


def q(**kw):
    base = {"text": "Work out $2+3$", "answer": "$5$", "topic": "Number", "year": "Y7", "difficulty": 2}
    return b.read_question({**base, **kw})


class Questions(unittest.TestCase):
    def test_good_question_passes(self):
        self.assertIsNone(b.question_problem(q()))

    def test_bad_ones_say_why(self):
        for bad, word in [(q(text="a $ b"), "$"), (q(answer=""), "answer"), (q(year="Y12"), "year"),
                          (q(difficulty=9), "level"), (q(text="x [[a]] y"), "[["), (q(text="\\frac{1}{2 x"), "curly")]:
            self.assertIn(word, b.question_problem(bad))

    def test_control_characters_are_removed(self):
        self.assertEqual(b.clean("a\x00b\x07c", 10), "abc")


if __name__ == "__main__":
    unittest.main()
