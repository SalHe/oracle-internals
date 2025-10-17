import shutil
import os
import oracledb

from typing import Callable
from oracle_internals.cli.decorators import within_container
from oracle_internals.config import CONFIG
from oracle_internals.db import download_file_from_container, must_get_container


class CaseBase:
    def _prepare(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def _run(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def _cleanup(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def _get_archive_files(self, cursor: oracledb.Cursor) -> set[str]:
        cursor.execute("SELECT * FROM v$archived_log")
        return set(map(lambda x: x[2], cursor.fetchall()))

    def _archive_log_file_and_wait(self, cursor: oracledb.Cursor):
        files = self._get_archive_files(cursor)
        cursor.execute('alter system archive log current')
        new_files = self._get_archive_files(cursor)
        return list(new_files - files)

    def __run_and_parse(self, get_connection: Callable[[], oracledb.Connection]):
        # 执行用例
        with get_connection() as connection:
            with connection.cursor() as cursor:
                # Prepare environment
                switched = self._archive_log_file_and_wait(cursor)
                assert len(switched) > 0

                # recorded operations
                self._run(get_connection)

                # Clean up
                redo_file = self._archive_log_file_and_wait(cursor)[0]

                # Get path of trace file
                sql = "select value from v$diag_info where name='Default Trace File'"
                cursor.execute(sql)

                diag_file = cursor.fetchone()
                assert diag_file is not None
                diag_file = diag_file[0]
                must_get_container().exec_run(f'truncate -s 0  {diag_file}')

                cursor.execute(f"ALTER SYSTEM DUMP LOGFILE '{redo_file}'")

        save_dir = os.path.join(os.path.abspath(
            CONFIG['outputs']['dumped']),
            self.__class__.__module__,
            self.__class__.__qualname__,
        )
        if os.path.exists(save_dir):
            shutil.rmtree(save_dir)
        os.makedirs(save_dir, exist_ok=True)

        file_trace = os.path.join(save_dir, os.path.basename(diag_file))
        download_file_from_container(diag_file, file_trace)
        os.symlink(file_trace, os.path.join(save_dir, 'redo.txt'))

        file_redo = os.path.join(save_dir, os.path.basename(redo_file))
        download_file_from_container(redo_file, file_redo)
        os.symlink(file_redo, os.path.join(save_dir, 'redo.dbf'))

    # Make scope of container as small as possible, to avoid effects from other (background) sessions.
    @within_container
    def test_run(self, get_connection: Callable[[], oracledb.Connection]):
        self._prepare(get_connection)
        try:
            self.__run_and_parse(get_connection)
        finally:
            self._cleanup(get_connection)
