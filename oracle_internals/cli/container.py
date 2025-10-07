
import docker
from oracle_internals.config import CONFIG
from oracle_internals.db import get_container


def remove_container(args):
    existing = get_container()
    if existing is None:
        return

    existing.stop()
    existing.remove()


def init_container(args):
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
