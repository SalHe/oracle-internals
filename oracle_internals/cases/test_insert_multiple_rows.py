from oracle_internals.cases.test_base import CaseBase


class TestInsertMultipleRows(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_multi (c1 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                # use INSERT ALL to create a multi-row insert operation
                cursor.execute("""
                    INSERT ALL
                      INTO t_multi VALUES (1)
                      INTO t_multi VALUES (2)
                      INTO t_multi VALUES (3)
                      INTO t_multi VALUES (4)
                    SELECT * FROM dual
                """)
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_multi')


class TestInsertMultipleRows2(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_multi (c1 number, c2 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                # use INSERT ALL to create a multi-row insert operation
                cursor.execute("""
                    INSERT ALL
                      INTO t_multi VALUES (1, 2)
                      INTO t_multi VALUES (2, 2)
                      INTO t_multi VALUES (null, 2)
                      INTO t_multi VALUES (4, 2)
                    SELECT * FROM dual
                """)
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_multi')
