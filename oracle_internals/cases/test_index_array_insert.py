from oracle_internals.cases.test_base import CaseBase


class TestIndexArrayInsertAndDelete(CaseBase):
    """INSERT ... SELECT and DELETE on an indexed table.

    `INSERT ... SELECT` uses the array-insert path, which folds several index
    keys into a single change (`REDO: 0x20 ARRAY` with `number of keys` and a
    `slots` list) instead of emitting one 10.2 (kdxlin) per row. The DELETE
    exercises 10.4 (kdxlde), the delete-leaf-row counterpart.
    """

    def _prepare(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('create table t_idx_arr (c1 number, c2 varchar2(10))')
                cursor.execute('create index idx_t_idx_arr on t_idx_arr (c1)')

    def _run(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute("""
                    insert into t_idx_arr (c1, c2)
                    select level, 'x' from dual connect by level <= 10
                """)
                cursor.execute('delete from t_idx_arr where c1 in (1, 2, 3)')
                connection.commit()

    def _cleanup(self, get_connection):
        with get_connection() as connection:
            with connection.cursor() as cursor:
                cursor.execute('drop table t_idx_arr')
