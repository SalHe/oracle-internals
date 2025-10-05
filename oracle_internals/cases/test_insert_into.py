from oracle_internals.cases.test_base import CaseBase


class TestSimpleInsertInto(CaseBase):
    """
    开启事务，插入一条数据后，立马提交。
    """
    
    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t1 (c1 number)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                connection.begin()
                cursor.execute('insert into t1 values (1)')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t1')
