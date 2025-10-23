import pytest
import oracle_internals.cases
from oracle_internals.db import must_get_container


def collect_case(args):
    py_args = args.pytest_args[1:]
    py_args.extend(['--pyargs', oracle_internals.cases.__name__])

    must_get_container().stop()

    pytest.main(py_args)
