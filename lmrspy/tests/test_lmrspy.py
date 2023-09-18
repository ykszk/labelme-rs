import unittest
from pathlib import Path

import lmrspy


class TestStringMethods(unittest.TestCase):

    def test_validation(self):
        json_path = str(Path(__file__).parent / '../../app/tests/test.json')
        rules = ['TL==1', 'TL>0']
        flags = []
        ignores = []
        validator = lmrspy.Validator(rules, flags, ignores)
        self.assertTrue(validator.validate_json(json_path))

        rules = ['TL==1', 'TL>0']
        validator = lmrspy.Validator(rules, flags, ['f1'])
        self.assertFalse(validator.validate_json(json_path))

        rules = ['TL==2']
        validator = lmrspy.Validator(rules, flags, ignores)
        with self.assertRaises(ValueError):
            validator.validate_json(json_path)

if __name__ == '__main__':
    unittest.main()