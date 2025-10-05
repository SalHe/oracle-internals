from datetime import datetime
import docker
import argparse
import pytest
import yaml
import subprocess

import oracle_internals.cases
from oracle_internals.config import CONFIG
from oracle_internals.db import get_container


def _remove_container(args):
    existing = get_container()
    if existing is None:
        return

    existing.stop()
    existing.remove()


def _init_container(args):
    docker_client = docker.from_env()
    container = docker_client.containers.run(
        image=CONFIG['container']['image'],
        name=CONFIG['container']['name'],
        detach=True,
        environment={
            'ORACLE_SID': 'ORCLINTERNAL',
            'ENABLE_ARCHIVELOG': 'true',
            'ORACLE_PWD': CONFIG['container']['db-password'],
        },
        ports={
            '1521/tcp': CONFIG['container'].get('port', 1521),
        },
    )
    for log in container.logs(stream=True):
        log = log.decode().rstrip()
        print(log)
        if CONFIG['container']['ready-tag'] == log:
            break

    container.stop()


def _collect(args):
    container = get_container()
    if container is None:
        raise RuntimeError('容器未初始化，请先运行 init 命令')

    py_args = args.pytest_args[1:]
    py_args.extend(['--pyargs', oracle_internals.cases.__name__])
    py_args.extend(
        [
            '--alluredir',
            CONFIG['outputs'].get('allure', './allure-results')
        ]
    )

    if container.status != 'running':
        container.start()
        for log in container.logs(stream=True, since=int(datetime.now().timestamp())):
            log = log.decode().rstrip()
            if CONFIG['container']['ready-tag'] == log:
                break

    try:
        pytest.main(py_args)
    finally:
        container.stop()


def _report(args):
    host = args.host
    port = args.port
    allure_dir = CONFIG['outputs'].get('allure', './allure-results')
    print(f"启动 Allure 报告服务: http://{host}:{port}")
    subprocess.run([
        'allure', 'serve', allure_dir,
        '--host', host,
        '--port', str(port)
    ])


def main():
    parser = argparse.ArgumentParser('oracle-internal')
    parser.add_argument(
        '--config',
        required=False,
        help='指定配置文件路径',
        default='./oracle-internal.yaml',
    )

    subparsers = parser.add_subparsers(dest='command', required=True)

    parser_init = subparsers.add_parser('init', help='初始化，主要用于准备容器环境')
    parser_init.set_defaults(func=_init_container)

    parser_uninit = subparsers.add_parser('uninit', help='反初始化，将容器强制删除')
    parser_uninit.set_defaults(func=_remove_container)

    parser_collect = subparsers.add_parser('collect', help='收集数据')
    parser_collect.set_defaults(func=_collect)
    parser_collect.add_argument(
        'pytest_args', nargs=argparse.REMAINDER, help='传递给 pytest 的参数')

    parser_report = subparsers.add_parser('report', help='启动 Allure 报告服务')
    parser_report.set_defaults(func=_report)
    parser_report.add_argument(
        '--host', default='127.0.0.1', help='监听地址，默认所有地址')
    parser_report.add_argument(
        '--port', type=int, default=2992, help='监听端口')

    args = parser.parse_args()

    with open(args.config) as f:
        CONFIG.update(yaml.safe_load(f))

    return args.func(args)
