import io
import tarfile
import docker
import oracledb
from docker.models.containers import Container
from oracle_internals.config import CONFIG


def get_container() -> Container | None:
    docker_client = docker.from_env()
    existing = docker_client.containers.list(
        all=True,
        filters={"name": CONFIG['container']['name']},
    )
    if existing:
        assert len(existing) == 1
        existing = existing[0]
        return existing

    return None


def must_get_container() -> Container:
    container = get_container()
    if container is None:
        raise RuntimeError('容器未初始化，请先运行 init 命令')
    return container


def db_connection():
    return oracledb.connect(user="sys", password=CONFIG['container']['db-password'], dsn="localhost:1521/ORCLINTERNAL", mode=oracledb.SYSDBA)


def get_file_content(container_path: str) -> bytes:
    container = must_get_container()
    stream, _ = container.get_archive(container_path)
    fileobj = io.BytesIO()
    for chunk in stream:
        fileobj.write(chunk)
    fileobj.seek(0)
    with tarfile.open(fileobj=fileobj) as tar:
        member = tar.getmember(container_path.split('/')[-1])
        stream = tar.extractfile(member)
        assert stream is not None
        return stream.read()  # type: ignore


def download_file_from_container(container_path: str, local_path: str):
    container = must_get_container()
    stream, _ = container.get_archive(container_path)
    fileobj = io.BytesIO()
    for chunk in stream:
        fileobj.write(chunk)
    fileobj.seek(0)
    with tarfile.open(fileobj=fileobj) as tar:
        member = tar.getmember(container_path.split('/')[-1])
        with open(local_path, 'wb') as f:
            stream = tar.extractfile(member)
            assert stream is not None
            f.write(stream.read())
