from typing import Callable

import oracledb

from oracle_internals.db import get_container, get_file_content


class CaseBase:
    def _prepare(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def _run(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def _cleanup(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def __run_and_parse(self, get_connection: Callable[[], oracledb.Connection]):
        # 执行用例
        with get_connection() as connection:
            with connection.cursor() as cursor:
                # Prepare environment
                cursor.execute('ALTER SYSTEM SWITCH LOGFILE')

                # recorded operations
                self._run(get_connection)

                # Clean up
                cursor.execute('ALTER SYSTEM SWITCH LOGFILE')

                cursor.execute("SELECT * FROM v$archived_log")
                last_one = None
                for row in cursor:
                    last_one = row

                assert last_one is not None
                redo_file = last_one[2]

        # 收集诊断信息
        with get_connection() as connection:
            with connection.cursor() as cursor:
                sql = "select value from v$diag_info where name='Default Trace File'"
                cursor.execute(sql)

                diag_file = cursor.fetchone()
                assert diag_file is not None
                diag_file = diag_file[0]
                get_container().exec_run(  # type: ignore
                    f'rm -f {diag_file}')

                cursor.execute(
                    f"ALTER SYSTEM DUMP LOGFILE '{redo_file}'")

        # 输出结果
        print("==============================================================")
        print(get_file_content(diag_file).decode())

    def test_run(self, get_connection: Callable[[], oracledb.Connection]):
        self._prepare(get_connection)
        try:
            self.__run_and_parse(get_connection)
        finally:
            self._cleanup(get_connection)
