from oracle_internals.cases.test_base import CaseBase


class TestSelectForUpdate(CaseBase):
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (id number, c1 number)')
                connection.begin()
                cursor.execute('insert into t1 values (1, 10)')
                cursor.execute('insert into t1 values (2, 20)')
                cursor.execute('insert into t1 values (3, 30)')
                connection.commit()

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute('select * from t1 where id = 1 for update')
                cursor.execute('select * from t1 where id = 2 for update wait 5')
                cursor.execute('select * from t1 where id = 3')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')

