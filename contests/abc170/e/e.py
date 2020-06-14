from bisect import bisect_left, bisect_right, insort
from itertools import chain, repeat, starmap
from math import log
from operator import add, eq, ne, gt, ge, lt, le, iadd
from textwrap import dedent
from collections import Sequence, MutableSequence
from functools import reduce
from functools import wraps
from sys import hexversion

try:
    from _thread import get_ident
except ImportError:
    from _dummy_thread import get_ident


def recursive_repr(fillvalue='...'):
    "Decorator to make a repr function return fillvalue for a recursive call."
    # pylint: disable=missing-docstring
    # Copied from reprlib in Python 3
    # https://hg.python.org/cpython/file/3.6/Lib/reprlib.py

    def decorating_function(user_function):
        repr_running = set()

        @wraps(user_function)
        def wrapper(self):
            key = id(self), get_ident()
            if key in repr_running:
                return fillvalue
            repr_running.add(key)
            try:
                result = user_function(self)
            finally:
                repr_running.discard(key)
            return result

        return wrapper

    return decorating_function


class SortedList(MutableSequence):
    """Sorted list is a sorted mutable sequence.
    Sorted list values are maintained in sorted order.
    Sorted list values must be comparable. The total ordering of values must
    not change while they are stored in the sorted list.
    Methods for adding values:
    * :func:`SortedList.add`
    * :func:`SortedList.update`
    * :func:`SortedList.__add__`
    * :func:`SortedList.__iadd__`
    * :func:`SortedList.__mul__`
    * :func:`SortedList.__imul__`
    Methods for removing values:
    * :func:`SortedList.clear`
    * :func:`SortedList.discard`
    * :func:`SortedList.remove`
    * :func:`SortedList.pop`
    * :func:`SortedList.__delitem__`
    Methods for looking up values:
    * :func:`SortedList.bisect_left`
    * :func:`SortedList.bisect_right`
    * :func:`SortedList.count`
    * :func:`SortedList.index`
    * :func:`SortedList.__contains__`
    * :func:`SortedList.__getitem__`
    Methods for iterating values:
    * :func:`SortedList.irange`
    * :func:`SortedList.islice`
    * :func:`SortedList.__iter__`
    * :func:`SortedList.__reversed__`
    Methods for miscellany:
    * :func:`SortedList.copy`
    * :func:`SortedList.__len__`
    * :func:`SortedList.__repr__`
    * :func:`SortedList._check`
    * :func:`SortedList._reset`
    Sorted lists use lexicographical ordering semantics when compared to other
    sequences.
    Some methods of mutable sequences are not supported and will raise
    not-implemented error.
    """
    DEFAULT_LOAD_FACTOR = 1000

    def __init__(self, iterable=None):
        """Initialize sorted list instance.
        Optional `iterable` argument provides an initial iterable of values to
        initialize the sorted list.
        Runtime complexity: `O(n*log(n))`
        >>> sl = SortedList()
        >>> sl
        SortedList([])
        >>> sl = SortedList([3, 1, 2, 5, 4])
        >>> sl
        SortedList([1, 2, 3, 4, 5])
        :param iterable: initial values (optional)
        """
        self._len = 0
        self._load = self.DEFAULT_LOAD_FACTOR
        self._lists = []
        self._maxes = []
        self._index = []
        self._offset = 0

        if iterable is not None:
            self._update(iterable)

    def __new__(cls, iterable=None):  # EDIT: 改変済み、key argumentを消した
        """Create new sorted list or sorted-key list instance.
        Optional `key`-function argument will return an instance of subtype
        :class:`SortedKeyList`.
        >>> sl = SortedList()
        >>> isinstance(sl, SortedList)
        True
        :param iterable: initial values (optional)
        """
        # pylint: disable=unused-argument
        return object.__new__(cls)

    def _reset(self, load):
        """Reset sorted list load factor.
        The `load` specifies the load-factor of the list. The default load
        factor of 1000 works well for lists from tens to tens-of-millions of
        values. Good practice is to use a value that is the cube root of the
        list size. With billions of elements, the best load factor depends on
        your usage. It's best to leave the load factor at the default until you
        start benchmarking.
        See :doc:`implementation` and :doc:`performance-scale` for more
        information.
        Runtime complexity: `O(n)`
        :param int load: load-factor for sorted list sublists
        """
        values = reduce(iadd, self._lists, [])
        self._clear()
        self._load = load
        self._update(values)

    def clear(self):
        """Remove all values from sorted list.
        Runtime complexity: `O(n)`
        """
        self._len = 0
        del self._lists[:]
        del self._maxes[:]
        del self._index[:]
        self._offset = 0

    _clear = clear

    def add(self, value):
        """Add `value` to sorted list.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList()
        >>> sl.add(3)
        >>> sl.add(1)
        >>> sl.add(2)
        >>> sl
        SortedList([1, 2, 3])
        :param value: value to add to sorted list
        """
        _lists = self._lists
        _maxes = self._maxes

        if _maxes:
            pos = bisect_right(_maxes, value)

            if pos == len(_maxes):
                pos -= 1
                _lists[pos].append(value)
                _maxes[pos] = value
            else:
                insort(_lists[pos], value)

            self._expand(pos)
        else:
            _lists.append([value])
            _maxes.append(value)

        self._len += 1

    def _expand(self, pos):
        """Split sublists with length greater than double the load-factor.
        Updates the index when the sublist length is less than double the load
        level. This requires incrementing the nodes in a traversal from the
        leaf node to the root. For an example traversal see
        ``SortedList._loc``.
        """
        _load = self._load
        _lists = self._lists
        _index = self._index

        if len(_lists[pos]) > (_load << 1):
            _maxes = self._maxes

            _lists_pos = _lists[pos]
            half = _lists_pos[_load:]
            del _lists_pos[_load:]
            _maxes[pos] = _lists_pos[-1]

            _lists.insert(pos + 1, half)
            _maxes.insert(pos + 1, half[-1])

            del _index[:]
        else:
            if _index:
                child = self._offset + pos
                while child:
                    _index[child] += 1
                    child = (child - 1) >> 1
                _index[0] += 1

    def update(self, iterable):
        """Update sorted list by adding all values from `iterable`.
        Runtime complexity: `O(k*log(n))` -- approximate.
        >>> sl = SortedList()
        >>> sl.update([3, 1, 2])
        >>> sl
        SortedList([1, 2, 3])
        :param iterable: iterable of values to add
        """
        _lists = self._lists
        _maxes = self._maxes
        values = sorted(iterable)

        if _maxes:
            if len(values) * 4 >= self._len:
                values.extend(chain.from_iterable(_lists))
                values.sort()
                self._clear()
            else:
                _add = self.add
                for val in values:
                    _add(val)
                return

        _load = self._load
        _lists.extend(values[pos:(pos + _load)]
                      for pos in range(0, len(values), _load))
        _maxes.extend(sublist[-1] for sublist in _lists)
        self._len = len(values)
        del self._index[:]

    _update = update

    def __contains__(self, value):
        """Return true if `value` is an element of the sorted list.
        ``sl.__contains__(value)`` <==> ``value in sl``
        Runtime complexity: `O(log(n))`
        >>> sl = SortedList([1, 2, 3, 4, 5])
        >>> 3 in sl
        True
        :param value: search for value in sorted list
        :return: true if `value` in sorted list
        """
        _maxes = self._maxes

        if not _maxes:
            return False

        pos = bisect_left(_maxes, value)

        if pos == len(_maxes):
            return False

        _lists = self._lists
        idx = bisect_left(_lists[pos], value)

        return _lists[pos][idx] == value

    def discard(self, value):
        """Remove `value` from sorted list if it is a member.
        If `value` is not a member, do nothing.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList([1, 2, 3, 4, 5])
        >>> sl.discard(5)
        >>> sl.discard(0)
        >>> sl == [1, 2, 3, 4]
        True
        :param value: `value` to discard from sorted list
        """
        _maxes = self._maxes

        if not _maxes:
            return

        pos = bisect_left(_maxes, value)

        if pos == len(_maxes):
            return

        _lists = self._lists
        idx = bisect_left(_lists[pos], value)

        if _lists[pos][idx] == value:
            self._delete(pos, idx)

    def remove(self, value):
        """Remove `value` from sorted list; `value` must be a member.
        If `value` is not a member, raise ValueError.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList([1, 2, 3, 4, 5])
        >>> sl.remove(5)
        >>> sl == [1, 2, 3, 4]
        True
        >>> sl.remove(0)
        Traceback (most recent call last):
          ...
        ValueError: 0 not in list
        :param value: `value` to remove from sorted list
        :raises ValueError: if `value` is not in sorted list
        """
        _maxes = self._maxes

        if not _maxes:
            raise ValueError('{0!r} not in list'.format(value))

        pos = bisect_left(_maxes, value)

        if pos == len(_maxes):
            raise ValueError('{0!r} not in list'.format(value))

        _lists = self._lists
        idx = bisect_left(_lists[pos], value)

        if _lists[pos][idx] == value:
            self._delete(pos, idx)
        else:
            raise ValueError('{0!r} not in list'.format(value))

    def _delete(self, pos, idx):
        """Delete value at the given `(pos, idx)`.
        Combines lists that are less than half the load level.
        Updates the index when the sublist length is more than half the load
        level. This requires decrementing the nodes in a traversal from the
        leaf node to the root. For an example traversal see
        ``SortedList._loc``.
        :param int pos: lists index
        :param int idx: sublist index
        """
        _lists = self._lists
        _maxes = self._maxes
        _index = self._index

        _lists_pos = _lists[pos]

        del _lists_pos[idx]
        self._len -= 1

        len_lists_pos = len(_lists_pos)

        if len_lists_pos > (self._load >> 1):
            _maxes[pos] = _lists_pos[-1]

            if _index:
                child = self._offset + pos
                while child > 0:
                    _index[child] -= 1
                    child = (child - 1) >> 1
                _index[0] -= 1
        elif len(_lists) > 1:
            if not pos:
                pos += 1

            prev = pos - 1
            _lists[prev].extend(_lists[pos])
            _maxes[prev] = _lists[prev][-1]

            del _lists[pos]
            del _maxes[pos]
            del _index[:]

            self._expand(prev)
        elif len_lists_pos:
            _maxes[pos] = _lists_pos[-1]
        else:
            del _lists[pos]
            del _maxes[pos]
            del _index[:]

    def _loc(self, pos, idx):
        """Convert an index pair (lists index, sublist index) into a single
        index number that corresponds to the position of the value in the
        sorted list.
        Many queries require the index be built. Details of the index are
        described in ``SortedList._build_index``.
        Indexing requires traversing the tree from a leaf node to the root. The
        parent of each node is easily computable at ``(pos - 1) // 2``.
        Left-child nodes are always at odd indices and right-child nodes are
        always at even indices.
        When traversing up from a right-child node, increment the total by the
        left-child node.
        The final index is the sum from traversal and the index in the sublist.
        For example, using the index from ``SortedList._build_index``::
            _index = 14 5 9 3 2 4 5
            _offset = 3
        Tree::
                 14
              5      9
            3   2  4   5
        Converting an index pair (2, 3) into a single index involves iterating
        like so:
        1. Starting at the leaf node: offset + alpha = 3 + 2 = 5. We identify
           the node as a left-child node. At such nodes, we simply traverse to
           the parent.
        2. At node 9, position 2, we recognize the node as a right-child node
           and accumulate the left-child in our total. Total is now 5 and we
           traverse to the parent at position 0.
        3. Iteration ends at the root.
        The index is then the sum of the total and sublist index: 5 + 3 = 8.
        :param int pos: lists index
        :param int idx: sublist index
        :return: index in sorted list
        """
        if not pos:
            return idx

        _index = self._index

        if not _index:
            self._build_index()

        total = 0

        # Increment pos to point in the index to len(self._lists[pos]).

        pos += self._offset

        # Iterate until reaching the root of the index tree at pos = 0.

        while pos:

            # Right-child nodes are at odd indices. At such indices
            # account the total below the left child node.

            if not pos & 1:
                total += _index[pos - 1]

            # Advance pos to the parent node.

            pos = (pos - 1) >> 1

        return total + idx

    def _pos(self, idx):
        """Convert an index into an index pair (lists index, sublist index)
        that can be used to access the corresponding lists position.
        Many queries require the index be built. Details of the index are
        described in ``SortedList._build_index``.
        Indexing requires traversing the tree to a leaf node. Each node has two
        children which are easily computable. Given an index, pos, the
        left-child is at ``pos * 2 + 1`` and the right-child is at ``pos * 2 +
        2``.
        When the index is less than the left-child, traversal moves to the
        left sub-tree. Otherwise, the index is decremented by the left-child
        and traversal moves to the right sub-tree.
        At a child node, the indexing pair is computed from the relative
        position of the child node as compared with the offset and the remaining
        index.
        For example, using the index from ``SortedList._build_index``::
            _index = 14 5 9 3 2 4 5
            _offset = 3
        Tree::
                 14
              5      9
            3   2  4   5
        Indexing position 8 involves iterating like so:
        1. Starting at the root, position 0, 8 is compared with the left-child
           node (5) which it is greater than. When greater the index is
           decremented and the position is updated to the right child node.
        2. At node 9 with index 3, we again compare the index to the left-child
           node with value 4. Because the index is the less than the left-child
           node, we simply traverse to the left.
        3. At node 4 with index 3, we recognize that we are at a leaf node and
           stop iterating.
        4. To compute the sublist index, we subtract the offset from the index
           of the leaf node: 5 - 3 = 2. To compute the index in the sublist, we
           simply use the index remaining from iteration. In this case, 3.
        The final index pair from our example is (2, 3) which corresponds to
        index 8 in the sorted list.
        :param int idx: index in sorted list
        :return: (lists index, sublist index) pair
        """
        if idx < 0:
            last_len = len(self._lists[-1])

            if (-idx) <= last_len:
                return len(self._lists) - 1, last_len + idx

            idx += self._len

            if idx < 0:
                raise IndexError('list index out of range')
        elif idx >= self._len:
            raise IndexError('list index out of range')

        if idx < len(self._lists[0]):
            return 0, idx

        _index = self._index

        if not _index:
            self._build_index()

        pos = 0
        child = 1
        len_index = len(_index)

        while child < len_index:
            index_child = _index[child]

            if idx < index_child:
                pos = child
            else:
                idx -= index_child
                pos = child + 1

            child = (pos << 1) + 1

        return (pos - self._offset, idx)

    def _build_index(self):
        """Build a positional index for indexing the sorted list.
        Indexes are represented as binary trees in a dense array notation
        similar to a binary heap.
        For example, given a lists representation storing integers::
            0: [1, 2, 3]
            1: [4, 5]
            2: [6, 7, 8, 9]
            3: [10, 11, 12, 13, 14]
        The first transformation maps the sub-lists by their length. The
        first row of the index is the length of the sub-lists::
            0: [3, 2, 4, 5]
        Each row after that is the sum of consecutive pairs of the previous
        row::
            1: [5, 9]
            2: [14]
        Finally, the index is built by concatenating these lists together::
            _index = [14, 5, 9, 3, 2, 4, 5]
        An offset storing the start of the first row is also stored::
            _offset = 3
        When built, the index can be used for efficient indexing into the list.
        See the comment and notes on ``SortedList._pos`` for details.
        """
        row0 = list(map(len, self._lists))

        if len(row0) == 1:
            self._index[:] = row0
            self._offset = 0
            return

        head = iter(row0)
        tail = iter(head)
        row1 = list(starmap(add, zip(head, tail)))

        if len(row0) & 1:
            row1.append(row0[-1])

        if len(row1) == 1:
            self._index[:] = row1 + row0
            self._offset = 1
            return

        size = 2 ** (int(log(len(row1) - 1, 2)) + 1)
        row1.extend(repeat(0, size - len(row1)))
        tree = [row0, row1]

        while len(tree[-1]) > 1:
            head = iter(tree[-1])
            tail = iter(head)
            row = list(starmap(add, zip(head, tail)))
            tree.append(row)

        reduce(iadd, reversed(tree), self._index)
        self._offset = size * 2 - 1

    def __delitem__(self, index):
        """Remove value at `index` from sorted list.
        ``sl.__delitem__(index)`` <==> ``del sl[index]``
        Supports slicing.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList('abcde')
        >>> del sl[2]
        >>> sl
        SortedList(['a', 'b', 'd', 'e'])
        >>> del sl[:2]
        >>> sl
        SortedList(['d', 'e'])
        :param index: integer or slice for indexing
        :raises IndexError: if index out of range
        """
        if isinstance(index, slice):
            start, stop, step = index.indices(self._len)

            if step == 1 and start < stop:
                if start == 0 and stop == self._len:
                    return self._clear()
                elif self._len <= 8 * (stop - start):
                    values = self._getitem(slice(None, start))
                    if stop < self._len:
                        values += self._getitem(slice(stop, None))
                    self._clear()
                    return self._update(values)

            indices = range(start, stop, step)

            # Delete items from greatest index to least so
            # that the indices remain valid throughout iteration.

            if step > 0:
                indices = reversed(indices)

            _pos, _delete = self._pos, self._delete

            for index in indices:
                pos, idx = _pos(index)
                _delete(pos, idx)
        else:
            pos, idx = self._pos(index)
            self._delete(pos, idx)

    def __getitem__(self, index):
        """Lookup value at `index` in sorted list.
        ``sl.__getitem__(index)`` <==> ``sl[index]``
        Supports slicing.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList('abcde')
        >>> sl[1]
        'b'
        >>> sl[-1]
        'e'
        >>> sl[2:5]
        ['c', 'd', 'e']
        :param index: integer or slice for indexing
        :return: value or list of values
        :raises IndexError: if index out of range
        """
        _lists = self._lists

        if isinstance(index, slice):
            start, stop, step = index.indices(self._len)

            if step == 1 and start < stop:
                if start == 0 and stop == self._len:
                    return reduce(iadd, self._lists, [])

                start_pos, start_idx = self._pos(start)

                if stop == self._len:
                    stop_pos = len(_lists) - 1
                    stop_idx = len(_lists[stop_pos])
                else:
                    stop_pos, stop_idx = self._pos(stop)

                if start_pos == stop_pos:
                    return _lists[start_pos][start_idx:stop_idx]

                prefix = _lists[start_pos][start_idx:]
                middle = _lists[(start_pos + 1):stop_pos]
                result = reduce(iadd, middle, prefix)
                result += _lists[stop_pos][:stop_idx]

                return result

            if step == -1 and start > stop:
                result = self._getitem(slice(stop + 1, start + 1))
                result.reverse()
                return result

            # Return a list because a negative step could
            # reverse the order of the items and this could
            # be the desired behavior.

            indices = range(start, stop, step)
            return list(self._getitem(index) for index in indices)
        else:
            if self._len:
                if index == 0:
                    return _lists[0][0]
                elif index == -1:
                    return _lists[-1][-1]
            else:
                raise IndexError('list index out of range')

            if 0 <= index < len(_lists[0]):
                return _lists[0][index]

            len_last = len(_lists[-1])

            if -len_last < index < 0:
                return _lists[-1][len_last + index]

            pos, idx = self._pos(index)
            return _lists[pos][idx]

    _getitem = __getitem__

    def __setitem__(self, index, value):
        """Raise not-implemented error.
        ``sl.__setitem__(index, value)`` <==> ``sl[index] = value``
        :raises NotImplementedError: use ``del sl[index]`` and
            ``sl.add(value)`` instead
        """
        message = 'use ``del sl[index]`` and ``sl.add(value)`` instead'
        raise NotImplementedError(message)

    def __iter__(self):
        """Return an iterator over the sorted list.
        ``sl.__iter__()`` <==> ``iter(sl)``
        Iterating the sorted list while adding or deleting values may raise a
        :exc:`RuntimeError` or fail to iterate over all values.
        """
        return chain.from_iterable(self._lists)

    def __reversed__(self):
        """Return a reverse iterator over the sorted list.
        ``sl.__reversed__()`` <==> ``reversed(sl)``
        Iterating the sorted list while adding or deleting values may raise a
        :exc:`RuntimeError` or fail to iterate over all values.
        """
        return chain.from_iterable(map(reversed, reversed(self._lists)))

    def reverse(self):
        """Raise not-implemented error.
        Sorted list maintains values in ascending sort order. Values may not be
        reversed in-place.
        Use ``reversed(sl)`` for an iterator over values in descending sort
        order.
        Implemented to override `MutableSequence.reverse` which provides an
        erroneous default implementation.
        :raises NotImplementedError: use ``reversed(sl)`` instead
        """
        raise NotImplementedError('use ``reversed(sl)`` instead')

    def islice(self, start=None, stop=None, reverse=False):
        """Return an iterator that slices sorted list from `start` to `stop`.
        The `start` and `stop` index are treated inclusive and exclusive,
        respectively.
        Both `start` and `stop` default to `None` which is automatically
        inclusive of the beginning and end of the sorted list.
        When `reverse` is `True` the values are yielded from the iterator in
        reverse order; `reverse` defaults to `False`.
        >>> sl = SortedList('abcdefghij')
        >>> it = sl.islice(2, 6)
        >>> list(it)
        ['c', 'd', 'e', 'f']
        :param int start: start index (inclusive)
        :param int stop: stop index (exclusive)
        :param bool reverse: yield values in reverse order
        :return: iterator
        """
        _len = self._len

        if not _len:
            return iter(())

        start, stop, _ = slice(start, stop).indices(self._len)

        if start >= stop:
            return iter(())

        _pos = self._pos

        min_pos, min_idx = _pos(start)

        if stop == _len:
            max_pos = len(self._lists) - 1
            max_idx = len(self._lists[-1])
        else:
            max_pos, max_idx = _pos(stop)

        return self._islice(min_pos, min_idx, max_pos, max_idx, reverse)

    def _islice(self, min_pos, min_idx, max_pos, max_idx, reverse):
        """Return an iterator that slices sorted list using two index pairs.
        The index pairs are (min_pos, min_idx) and (max_pos, max_idx), the
        first inclusive and the latter exclusive. See `_pos` for details on how
        an index is converted to an index pair.
        When `reverse` is `True`, values are yielded from the iterator in
        reverse order.
        """
        _lists = self._lists

        if min_pos > max_pos:
            return iter(())

        if min_pos == max_pos:
            if reverse:
                indices = reversed(range(min_idx, max_idx))
                return map(_lists[min_pos].__getitem__, indices)

            indices = range(min_idx, max_idx)
            return map(_lists[min_pos].__getitem__, indices)

        next_pos = min_pos + 1

        if next_pos == max_pos:
            if reverse:
                min_indices = range(min_idx, len(_lists[min_pos]))
                max_indices = range(max_idx)
                return chain(
                    map(_lists[max_pos].__getitem__, reversed(max_indices)),
                    map(_lists[min_pos].__getitem__, reversed(min_indices)),
                )

            min_indices = range(min_idx, len(_lists[min_pos]))
            max_indices = range(max_idx)
            return chain(
                map(_lists[min_pos].__getitem__, min_indices),
                map(_lists[max_pos].__getitem__, max_indices),
            )

        if reverse:
            min_indices = range(min_idx, len(_lists[min_pos]))
            sublist_indices = range(next_pos, max_pos)
            sublists = map(_lists.__getitem__, reversed(sublist_indices))
            max_indices = range(max_idx)
            return chain(
                map(_lists[max_pos].__getitem__, reversed(max_indices)),
                chain.from_iterable(map(reversed, sublists)),
                map(_lists[min_pos].__getitem__, reversed(min_indices)),
            )

        min_indices = range(min_idx, len(_lists[min_pos]))
        sublist_indices = range(next_pos, max_pos)
        sublists = map(_lists.__getitem__, sublist_indices)
        max_indices = range(max_idx)
        return chain(
            map(_lists[min_pos].__getitem__, min_indices),
            chain.from_iterable(sublists),
            map(_lists[max_pos].__getitem__, max_indices),
        )

    def irange(self, minimum=None, maximum=None, inclusive=(True, True),
               reverse=False):
        """Create an iterator of values between `minimum` and `maximum`.
        Both `minimum` and `maximum` default to `None` which is automatically
        inclusive of the beginning and end of the sorted list.
        The argument `inclusive` is a pair of booleans that indicates whether
        the minimum and maximum ought to be included in the range,
        respectively. The default is ``(True, True)`` such that the range is
        inclusive of both minimum and maximum.
        When `reverse` is `True` the values are yielded from the iterator in
        reverse order; `reverse` defaults to `False`.
        >>> sl = SortedList('abcdefghij')
        >>> it = sl.irange('c', 'f')
        >>> list(it)
        ['c', 'd', 'e', 'f']
        :param minimum: minimum value to start iterating
        :param maximum: maximum value to stop iterating
        :param inclusive: pair of booleans
        :param bool reverse: yield values in reverse order
        :return: iterator
        """
        _maxes = self._maxes

        if not _maxes:
            return iter(())

        _lists = self._lists

        # Calculate the minimum (pos, idx) pair. By default this location
        # will be inclusive in our calculation.

        if minimum is None:
            min_pos = 0
            min_idx = 0
        else:
            if inclusive[0]:
                min_pos = bisect_left(_maxes, minimum)

                if min_pos == len(_maxes):
                    return iter(())

                min_idx = bisect_left(_lists[min_pos], minimum)
            else:
                min_pos = bisect_right(_maxes, minimum)

                if min_pos == len(_maxes):
                    return iter(())

                min_idx = bisect_right(_lists[min_pos], minimum)

        # Calculate the maximum (pos, idx) pair. By default this location
        # will be exclusive in our calculation.

        if maximum is None:
            max_pos = len(_maxes) - 1
            max_idx = len(_lists[max_pos])
        else:
            if inclusive[1]:
                max_pos = bisect_right(_maxes, maximum)

                if max_pos == len(_maxes):
                    max_pos -= 1
                    max_idx = len(_lists[max_pos])
                else:
                    max_idx = bisect_right(_lists[max_pos], maximum)
            else:
                max_pos = bisect_left(_maxes, maximum)

                if max_pos == len(_maxes):
                    max_pos -= 1
                    max_idx = len(_lists[max_pos])
                else:
                    max_idx = bisect_left(_lists[max_pos], maximum)

        return self._islice(min_pos, min_idx, max_pos, max_idx, reverse)

    def __len__(self):
        """Return the size of the sorted list.
        ``sl.__len__()`` <==> ``len(sl)``
        :return: size of sorted list
        """
        return self._len

    def bisect_left(self, value):
        """Return an index to insert `value` in the sorted list.
        If the `value` is already present, the insertion point will be before
        (to the left of) any existing values.
        Similar to the `bisect` module in the standard library.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList([10, 11, 12, 13, 14])
        >>> sl.bisect_left(12)
        2
        :param value: insertion index of value in sorted list
        :return: index
        """
        _maxes = self._maxes

        if not _maxes:
            return 0

        pos = bisect_left(_maxes, value)

        if pos == len(_maxes):
            return self._len

        idx = bisect_left(self._lists[pos], value)
        return self._loc(pos, idx)

    def bisect_right(self, value):
        """Return an index to insert `value` in the sorted list.
        Similar to `bisect_left`, but if `value` is already present, the
        insertion point will be after (to the right of) any existing values.
        Similar to the `bisect` module in the standard library.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList([10, 11, 12, 13, 14])
        >>> sl.bisect_right(12)
        3
        :param value: insertion index of value in sorted list
        :return: index
        """
        _maxes = self._maxes

        if not _maxes:
            return 0

        pos = bisect_right(_maxes, value)

        if pos == len(_maxes):
            return self._len

        idx = bisect_right(self._lists[pos], value)
        return self._loc(pos, idx)

    bisect = bisect_right
    _bisect_right = bisect_right

    def count(self, value):
        """Return number of occurrences of `value` in the sorted list.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList([1, 2, 2, 3, 3, 3, 4, 4, 4, 4])
        >>> sl.count(3)
        3
        :param value: value to count in sorted list
        :return: count
        """
        _maxes = self._maxes

        if not _maxes:
            return 0

        pos_left = bisect_left(_maxes, value)

        if pos_left == len(_maxes):
            return 0

        _lists = self._lists
        idx_left = bisect_left(_lists[pos_left], value)
        pos_right = bisect_right(_maxes, value)

        if pos_right == len(_maxes):
            return self._len - self._loc(pos_left, idx_left)

        idx_right = bisect_right(_lists[pos_right], value)

        if pos_left == pos_right:
            return idx_right - idx_left

        right = self._loc(pos_right, idx_right)
        left = self._loc(pos_left, idx_left)
        return right - left

    def copy(self):
        """Return a shallow copy of the sorted list.
        Runtime complexity: `O(n)`
        :return: new sorted list
        """
        return self.__class__(self)

    __copy__ = copy

    def append(self, value):
        """Raise not-implemented error.
        Implemented to override `MutableSequence.append` which provides an
        erroneous default implementation.
        :raises NotImplementedError: use ``sl.add(value)`` instead
        """
        raise NotImplementedError('use ``sl.add(value)`` instead')

    def extend(self, values):
        """Raise not-implemented error.
        Implemented to override `MutableSequence.extend` which provides an
        erroneous default implementation.
        :raises NotImplementedError: use ``sl.update(values)`` instead
        """
        raise NotImplementedError('use ``sl.update(values)`` instead')

    def insert(self, index, value):
        """Raise not-implemented error.
        :raises NotImplementedError: use ``sl.add(value)`` instead
        """
        raise NotImplementedError('use ``sl.add(value)`` instead')

    def pop(self, index=-1):
        """Remove and return value at `index` in sorted list.
        Raise :exc:`IndexError` if the sorted list is empty or index is out of
        range.
        Negative indices are supported.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList('abcde')
        >>> sl.pop()
        'e'
        >>> sl.pop(2)
        'c'
        >>> sl
        SortedList(['a', 'b', 'd'])
        :param int index: index of value (default -1)
        :return: value
        :raises IndexError: if index is out of range
        """
        if not self._len:
            raise IndexError('pop index out of range')

        _lists = self._lists

        if index == 0:
            val = _lists[0][0]
            self._delete(0, 0)
            return val

        if index == -1:
            pos = len(_lists) - 1
            loc = len(_lists[pos]) - 1
            val = _lists[pos][loc]
            self._delete(pos, loc)
            return val

        if 0 <= index < len(_lists[0]):
            val = _lists[0][index]
            self._delete(0, index)
            return val

        len_last = len(_lists[-1])

        if -len_last < index < 0:
            pos = len(_lists) - 1
            loc = len_last + index
            val = _lists[pos][loc]
            self._delete(pos, loc)
            return val

        pos, idx = self._pos(index)
        val = _lists[pos][idx]
        self._delete(pos, idx)
        return val

    def index(self, value, start=None, stop=None):
        """Return first index of value in sorted list.
        Raise ValueError if `value` is not present.
        Index must be between `start` and `stop` for the `value` to be
        considered present. The default value, None, for `start` and `stop`
        indicate the beginning and end of the sorted list.
        Negative indices are supported.
        Runtime complexity: `O(log(n))` -- approximate.
        >>> sl = SortedList('abcde')
        >>> sl.index('d')
        3
        >>> sl.index('z')
        Traceback (most recent call last):
          ...
        ValueError: 'z' is not in list
        :param value: value in sorted list
        :param int start: start index (default None, start of sorted list)
        :param int stop: stop index (default None, end of sorted list)
        :return: index of value
        :raises ValueError: if value is not present
        """
        _len = self._len

        if not _len:
            raise ValueError('{0!r} is not in list'.format(value))

        if start is None:
            start = 0
        if start < 0:
            start += _len
        if start < 0:
            start = 0

        if stop is None:
            stop = _len
        if stop < 0:
            stop += _len
        if stop > _len:
            stop = _len

        if stop <= start:
            raise ValueError('{0!r} is not in list'.format(value))

        _maxes = self._maxes
        pos_left = bisect_left(_maxes, value)

        if pos_left == len(_maxes):
            raise ValueError('{0!r} is not in list'.format(value))

        _lists = self._lists
        idx_left = bisect_left(_lists[pos_left], value)

        if _lists[pos_left][idx_left] != value:
            raise ValueError('{0!r} is not in list'.format(value))

        stop -= 1
        left = self._loc(pos_left, idx_left)

        if start <= left:
            if left <= stop:
                return left
        else:
            right = self._bisect_right(value) - 1

            if start <= right:
                return start

        raise ValueError('{0!r} is not in list'.format(value))

    def __add__(self, other):
        """Return new sorted list containing all values in both sequences.
        ``sl.__add__(other)`` <==> ``sl + other``
        Values in `other` do not need to be in sorted order.
        Runtime complexity: `O(n*log(n))`
        >>> sl1 = SortedList('bat')
        >>> sl2 = SortedList('cat')
        >>> sl1 + sl2
        SortedList(['a', 'a', 'b', 'c', 't', 't'])
        :param other: other iterable
        :return: new sorted list
        """
        values = reduce(iadd, self._lists, [])
        values.extend(other)
        return self.__class__(values)

    __radd__ = __add__

    def __iadd__(self, other):
        """Update sorted list with values from `other`.
        ``sl.__iadd__(other)`` <==> ``sl += other``
        Values in `other` do not need to be in sorted order.
        Runtime complexity: `O(k*log(n))` -- approximate.
        >>> sl = SortedList('bat')
        >>> sl += 'cat'
        >>> sl
        SortedList(['a', 'a', 'b', 'c', 't', 't'])
        :param other: other iterable
        :return: existing sorted list
        """
        self._update(other)
        return self

    def __mul__(self, num):
        """Return new sorted list with `num` shallow copies of values.
        ``sl.__mul__(num)`` <==> ``sl * num``
        Runtime complexity: `O(n*log(n))`
        >>> sl = SortedList('abc')
        >>> sl * 3
        SortedList(['a', 'a', 'a', 'b', 'b', 'b', 'c', 'c', 'c'])
        :param int num: count of shallow copies
        :return: new sorted list
        """
        values = reduce(iadd, self._lists, []) * num
        return self.__class__(values)

    __rmul__ = __mul__

    def __imul__(self, num):
        """Update the sorted list with `num` shallow copies of values.
        ``sl.__imul__(num)`` <==> ``sl *= num``
        Runtime complexity: `O(n*log(n))`
        >>> sl = SortedList('abc')
        >>> sl *= 3
        >>> sl
        SortedList(['a', 'a', 'a', 'b', 'b', 'b', 'c', 'c', 'c'])
        :param int num: count of shallow copies
        :return: existing sorted list
        """
        values = reduce(iadd, self._lists, []) * num
        self._clear()
        self._update(values)
        return self

    def __make_cmp(seq_op, symbol, doc):
        "Make comparator method."

        def comparer(self, other):
            "Compare method for sorted list and sequence."
            if not isinstance(other, Sequence):
                return NotImplemented

            self_len = self._len
            len_other = len(other)

            if self_len != len_other:
                if seq_op is eq:
                    return False
                if seq_op is ne:
                    return True

            for alpha, beta in zip(self, other):
                if alpha != beta:
                    return seq_op(alpha, beta)

            return seq_op(self_len, len_other)

        seq_op_name = seq_op.__name__
        comparer.__name__ = '__{0}__'.format(seq_op_name)
        doc_str = """Return true if and only if sorted list is {0} `other`.
        ``sl.__{1}__(other)`` <==> ``sl {2} other``
        Comparisons use lexicographical order as with sequences.
        Runtime complexity: `O(n)`
        :param other: `other` sequence
        :return: true if sorted list is {0} `other`
        """
        comparer.__doc__ = dedent(doc_str.format(doc, seq_op_name, symbol))
        return comparer

    __eq__ = __make_cmp(eq, '==', 'equal to')
    __ne__ = __make_cmp(ne, '!=', 'not equal to')
    __lt__ = __make_cmp(lt, '<', 'less than')
    __gt__ = __make_cmp(gt, '>', 'greater than')
    __le__ = __make_cmp(le, '<=', 'less than or equal to')
    __ge__ = __make_cmp(ge, '>=', 'greater than or equal to')
    __make_cmp = staticmethod(__make_cmp)

    def __reduce__(self):
        values = reduce(iadd, self._lists, [])
        return (type(self), (values,))

    @recursive_repr()
    def __repr__(self):
        """Return string representation of sorted list.
        ``sl.__repr__()`` <==> ``repr(sl)``
        :return: string representation
        """
        return '{0}({1!r})'.format(type(self).__name__, list(self))


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


class SegmentTree:
    def __init__(self, ls: list, segfunc, identity_element):
        '''
        セグ木
        一次元のリストlsを受け取り初期化する。O(len(ls))
        区間のルールはsegfuncによって定義される
        identity elementは単位元。e.g., 最小値を求めたい→inf, 和→0, 積→1, gcd→0
        [単位元](https://ja.wikipedia.org/wiki/%E5%8D%98%E4%BD%8D%E5%85%83)
        '''
        self.ide = identity_element
        self.func = segfunc
        self.n_origin = len(ls)
        self.num = 2 ** (self.n_origin - 1).bit_length()  # n以上の最小の2のべき乗
        self.tree = [self.ide] * (2 * self.num - 1)  # −1はぴったりに作るためだけど気にしないでいい
        for i, l in enumerate(ls):  # 木の葉に代入
            self.tree[i + self.num - 1] = l
        for i in range(self.num - 2, -1, -1):  # 子を束ねて親を更新
            self.tree[i] = segfunc(self.tree[2 * i + 1], self.tree[2 * i + 2])

    def __getitem__(self, idx):  # オリジナル要素にアクセスするためのもの
        if isinstance(idx, slice):
            start = idx.start if idx.start else 0
            stop = idx.stop if idx.stop else self.n_origin
            l = start + self.num - 1
            r = l + stop - start
            return self.tree[l:r:idx.step]
        elif isinstance(idx, int):
            i = idx + self.num - 1
            return self.tree[i]

    def update(self, i, x):
        '''
        i番目の要素をxに変更する(木の中間ノードも更新する) O(logN)
        '''
        i += self.num - 1
        self.tree[i] = x
        while i:  # 木を更新
            i = (i - 1) // 2
            self.tree[i] = self.func(self.tree[i * 2 + 1],
                                     self.tree[i * 2 + 2])

    def query(self, l, r):
        '''区間[l,r)に対するクエリをO(logN)で処理する。例えばその区間の最小値、最大値、gcdなど'''
        if r <= l:
            return ValueError('invalid index (l,rがありえないよ)')
        l += self.num
        r += self.num
        res_right = []
        res_left = []
        while l < r:  # 右から寄りながら結果を結合していくイメージ
            if l & 1:
                res_left.append(self.tree[l - 1])
                l += 1
            if r & 1:
                r -= 1
                res_right.append(self.tree[r - 1])
            l >>= 1
            r >>= 1
        res = self.ide
        # 左右の順序を保って結合
        for x in res_left:
            res = self.func(x, res)
        for x in reversed(res_right):
            res = self.func(res, x)
        return res


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right


N, Q = read_ints()
A, B = read_col(N)
C, D = read_col(Q)

# セグ木か？
# セグ木だと書き換えはOK
# 園児Cの抜けた部分の処理はどうする？
# 二分探索とdict配列でレートの順番を管理しておく?
# でも園児Cjがどこにいるかがわからない←それも管理しておけばいいじゃん
# sorted list使えばむりやり実装できないこともないが...

youchien = defaultdict(lambda: SortedList())  # 中身はレート
youji_to_en = {}
for i, (a, b) in enu(zip(A, B)):
    youchien[b].add(a)
    youji_to_en[i] = b


segtree = SegmentTree([INF] * (2 * 10 ** 5 + 1), min, INF)
# 初期化
for k, v in youchien.items():
    segtree.update(k, v[-1])  # k幼稚園を一番レートの高いものに

ans = []
for c, to in zip(C, D):
    c -= 1
    fr = youji_to_en[c]
    youji_to_en[c] = to
    youchien[fr].remove(A[c])
    youchien[to].add(A[c])
    if len(youchien[fr]):
        segtree.update(fr, youchien[fr][-1])
    else:
        segtree.update(fr, INF)
    segtree.update(to, youchien[to][-1])
    ans.append(segtree.tree[0])
print(*ans, sep='\n')
