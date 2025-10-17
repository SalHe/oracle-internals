import pytest
import oracle_internals.cases


def collect_case(args):
    py_args = args.pytest_args[1:]
    py_args.extend(['--pyargs', oracle_internals.cases.__name__])

    pytest.main(py_args)
