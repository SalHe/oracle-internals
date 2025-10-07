import argparse
import yaml

from oracle_internals.config import CONFIG
from oracle_internals.cli import redo, container, testing


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
    parser_init.set_defaults(func=container.init_container)

    parser_uninit = subparsers.add_parser('uninit', help='反初始化，将容器强制删除')
    parser_uninit.set_defaults(func=container.remove_container)

    parser_collect = subparsers.add_parser('collect', help='收集数据')
    parser_collect.set_defaults(func=testing.collect_case)
    parser_collect.add_argument(
        'pytest_args', nargs=argparse.REMAINDER, help='传递给 pytest 的参数')

    parser_report = subparsers.add_parser('report', help='启动 Allure 报告服务')
    parser_report.set_defaults(func=testing.open_report)
    parser_report.add_argument(
        '--host', default='127.0.0.1', help='监听地址，默认所有地址')
    parser_report.add_argument(
        '--port', type=int, default=2992, help='监听端口')

    parser_redo = subparsers.add_parser('redo', help='redo 文件相关操作')
    redo_subparsers = parser_redo.add_subparsers(
        dest='redo_command', required=True)

    parser_redo_ls = redo_subparsers.add_parser('ls', help='列举归档日志文件')
    parser_redo_ls.set_defaults(func=redo.list_redo_files)

    parser_redo_cp = redo_subparsers.add_parser('cp', help='复制指定 redo 文件到目标路径')
    parser_redo_cp.add_argument('src', help='容器内 redo 文件路径')
    parser_redo_cp.add_argument('dst', help='主机目标路径')
    parser_redo_cp.set_defaults(func=redo.copy_redo)

    parser_redo_dump = redo_subparsers.add_parser(
        'dump', help='dump redo 文件为文本')
    parser_redo_dump.add_argument('redo_file', help='redo 文件路径')
    parser_redo_dump.add_argument(
        '-l', '--local_file', help='redo 文件来自于宿主机', default=False, action='store_true')
    parser_redo_dump.set_defaults(func=redo.dump_redo)

    args = parser.parse_args()

    with open(args.config) as f:
        CONFIG.update(yaml.safe_load(f))

    return args.func(args)
