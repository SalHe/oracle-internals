from typing import Callable

import oracledb


class Case:
    def prepare(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def run(self, get_connection: Callable[[], oracledb.Connection]):
        pass

    def cleanup(self, get_connection: Callable[[], oracledb.Connection]):
        pass
