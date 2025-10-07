from datetime import datetime
import functools
from oracle_internals.config import CONFIG
from oracle_internals.db import must_get_container


def within_container(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        container = must_get_container()
        if container is None:
            raise RuntimeError('容器未初始化，请先运行 init 命令')

        started_from_me = False
        if container.status != 'running':
            started_from_me = True
            container.start()
            for log in container.logs(stream=True, since=int(datetime.now().timestamp())):
                log = log.decode().rstrip()
                if CONFIG['container']['ready-tag'] == log:
                    break

        try:
            return func(*args, **kwargs)
        finally:
            if started_from_me:
                container.stop()

    return wrapper
