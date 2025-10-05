from datetime import datetime
import docker
import argparse
import yaml

from oracle_internals.cases.all import get_all_cases
from oracle_internals.config import CONFIG
from oracle_internals.db import db_connection, get_container, get_file_content
from docker.models.containers import Container


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


def _run_all_case(container: Container):
    redo_file = None
    for c in get_all_cases():
        try:
            # 执行用例
            with db_connection() as connection:
                with connection.cursor() as cursor:
                    # Prepare environment
                    c.prepare(db_connection)
                    cursor.execute('ALTER SYSTEM SWITCH LOGFILE')

                    # recorded operations
                    c.run(db_connection)

                    # Clean up
                    cursor.execute('ALTER SYSTEM SWITCH LOGFILE')
                    c.cleanup(db_connection)

                    cursor.execute("SELECT * FROM v$archived_log")
                    last_one = None
                    for row in cursor:
                        last_one = row

                    assert last_one is not None
                    print('Last archived log:', last_one)
                    redo_file = last_one[2]

            # 收集诊断信息
            with db_connection() as connection:
                with connection.cursor() as cursor:
                    sql = "select value from v$diag_info where name='Default Trace File'"
                    cursor.execute(sql)

                    diag_file = cursor.fetchone()
                    assert diag_file is not None
                    diag_file = diag_file[0]
                    container.exec_run(f'rm -f {diag_file}')

                    cursor.execute(
                        f"ALTER SYSTEM DUMP LOGFILE '{redo_file}'")

            # 输出结果
            print("==============================================================")
            print(get_file_content(diag_file).decode())
        except Exception as e:
            print(f'用例 {c} 执行失败: {e}')
            raise


def _collect(args):
    container = get_container()
    if container is None:
        raise RuntimeError('容器未初始化，请先运行 init 命令')

    if container.status != 'running':
        container.start()
        for log in container.logs(stream=True, since=int(datetime.now().timestamp())):
            log = log.decode().rstrip()
            if CONFIG['container']['ready-tag'] == log:
                break

    try:
        _run_all_case(container)
    finally:
        container.stop()


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

    args = parser.parse_args()

    with open(args.config) as f:
        CONFIG.update(yaml.safe_load(f))

    return args.func(args)
