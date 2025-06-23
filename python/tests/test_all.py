import pytest
import secure_openaiapi


def test_sum_as_string():
    assert secure_openaiapi.sum_as_string(1, 1) == "2"
