from oracle_internals.cases.test_base import CaseBase


class TestInsertMultipleRows(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_multi_1 (c1 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                # use INSERT ALL to create a multi-row insert operation
                cursor.execute("""
                    INSERT ALL
                      INTO t_multi_1 VALUES (1)
                      INTO t_multi_1 VALUES (2)
                      INTO t_multi_1 VALUES (3)
                      INTO t_multi_1 VALUES (4)
                    SELECT * FROM dual
                """)
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_multi_1')


class TestInsertMultipleRows2(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_multi_2 (c1 number, c2 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                # use INSERT ALL to create a multi-row insert operation
                cursor.execute("""
                    INSERT ALL
                      INTO t_multi_2 VALUES (1, 2)
                      INTO t_multi_2 VALUES (2, 2)
                      INTO t_multi_2 VALUES (null, 2)
                      INTO t_multi_2 VALUES (4, 2)
                    SELECT * FROM dual
                """)
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_multi_2')

class TestInsertMultipleRows3(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_multi_3 (c1 number, c2 varchar(64), c3 varchar(8000))')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                # use INSERT ALL to create a multi-row insert operation
                cursor.execute("""
                    INSERT ALL
                      INTO t_multi_3 VALUES (1, 'aaaaaaaaa', lpad('x', 254, 'y'))
                      INTO t_multi_3 VALUES (2, 'aaaaaaaaa', lpad('x', 255, 'y'))
                      INTO t_multi_3 VALUES (3, 'aaaaaaaaa', lpad('x', 128, 'y'))
                      INTO t_multi_3 VALUES (4, 'aaaaaaaaa', lpad('x', 64, 'y'))
                      INTO t_multi_3 VALUES (5, 'aaaaaaaaa', lpad('x', 16, 'y'))
                      INTO t_multi_3 VALUES (6, 'aaaaaaaaa', lpad('x', 20, 'y'))
                      INTO t_multi_3 VALUES (null, null, 'xxyy')
                    SELECT * FROM dual
                """)
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_multi_3')
