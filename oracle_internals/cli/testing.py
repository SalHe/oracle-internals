import subprocess
import pytest
import oracle_internals.cases
from oracle_internals.cli.decorators import within_container
from oracle_internals.config import CONFIG


@within_container
def collect_case(args):
    py_args = args.pytest_args[1:]
    py_args.extend(['--pyargs', oracle_internals.cases.__name__])
    py_args.extend(
        [
            '--alluredir',
            CONFIG['outputs'].get('allure', './allure-results')
        ]
    )

    pytest.main(py_args)


def open_report(args):
    host = args.host
    port = args.port
    allure_dir = CONFIG['outputs'].get('allure', './allure-results')
    print(f"启动 Allure 报告服务: http://{host}:{port}")
    subprocess.run([
        'allure', 'serve', allure_dir,
        '--host', host,
        '--port', str(port)
    ])
