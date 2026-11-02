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


class FakeHandler:
    def __init__(self, token="", teacher=""):
        self.headers = {"X-Token": token, "X-Teacher": teacher}


class Api(unittest.TestCase):
    def setUp(self):
        import hashlib, tempfile
        self.dir = tempfile.mkdtemp()
        b.DATA_DIR = self.dir
        b.LEGACY_ADMIN = os.path.join(self.dir, "none.json")
        b.derive = lambda p, s: hashlib.sha256(s + p.encode()).hexdigest()
        b.time.sleep = lambda s: None
        b.SESSIONS.clear()
        b.save_user("boss", "Boss", "admin", "bosspass1")
        b.save_user("ed", "Ed", "editor", "editorpw1")

    def token(self, user, pw):
        status, r = b.api(FakeHandler(), "POST", "login", {"username": user, "password": pw})
        self.assertEqual(status, 200)
        return r["token"]

    def good(self, **kw):
        return {"text": "Work out $2+3$", "answer": "$5$", "topic": "Number", "year": "Y7", "difficulty": 2, **kw}

    def test_login_and_editing(self):
        self.assertEqual(b.api(FakeHandler(), "POST", "login", {"username": "ed", "password": "x"})[0], 403)
        self.assertEqual(b.api(FakeHandler(), "POST", "edit/save", self.good())[0], 403)
        h = FakeHandler(self.token("ed", "editorpw1"))
        status, r = b.api(h, "POST", "edit/save", self.good())
        self.assertEqual((status, r["id"]), (200, "staff-1"))
        b.api(h, "POST", "edit/save", self.good(id="staff-1", text="Work out $3+3$"))
        b.api(h, "POST", "edit/retire", {"target": "alg-1"})
        b.api(h, "POST", "edit/replace", self.good(target="alg-2"))
        live = b.api(FakeHandler(), "GET", "live.json", None)[1]
        self.assertEqual([q["text"] for q in live["myQuestions"]], ["Work out $3+3$", "Work out $2+3$"])
        self.assertEqual(live["removedQuestions"], ["alg-1", "alg-2"])
        b.api(h, "POST", "edit/restore", {"target": "alg-1"})
        b.api(h, "POST", "edit/delete", {"id": "staff-1"})
        live = b.api(FakeHandler(), "GET", "live.json", None)[1]
        self.assertEqual(live["removedQuestions"], ["alg-2"])
        self.assertEqual(len(live["myQuestions"]), 1)

    def test_only_admins_manage_logins(self):
        ed, boss = FakeHandler(self.token("ed", "editorpw1")), FakeHandler(self.token("boss", "bosspass1"))
        self.assertEqual(b.api(ed, "GET", "users", None)[0], 404)
        new = {"username": "kim", "name": "Kim", "role": "editor", "password": "kimpass12"}
        self.assertEqual(b.api(ed, "POST", "users/save", new)[0], 404)
        self.assertEqual(b.api(boss, "POST", "users/save", new)[0], 200)
        self.assertEqual(len(b.api(boss, "GET", "users", None)[1]["users"]), 3)

    def test_suggestions_need_approval_by_a_login(self):
        h = FakeHandler(teacher="Pat")
        self.assertEqual(b.api(h, "POST", "suggest/new", self.good())[0], 200)
        self.assertEqual(b.api(FakeHandler(), "GET", "live.json", None)[1]["myQuestions"], [])
        self.assertEqual(b.api(h, "POST", "review", {"id": 1, "decision": "approved"})[0], 403)
        ed = FakeHandler(self.token("ed", "editorpw1"))
        self.assertEqual(b.api(ed, "POST", "review", {"id": 1, "decision": "approved"})[0], 200)
        self.assertEqual(len(b.api(FakeHandler(), "GET", "live.json", None)[1]["myQuestions"]), 1)


if __name__ == "__main__":
    unittest.main()
