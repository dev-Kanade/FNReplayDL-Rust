import struct

class ReplayBuffer:
    def __init__(self):
        self.data = bytearray()

    def write_byte(self, val):
        self.data.append(val & 0xFF)

    def write_int32(self, val, offset=None):
        packed = struct.pack('<i', val)
        if offset is not None:
            self.data[offset:offset+4] = packed
        else:
            self.data.extend(packed)

    def write_int64(self, val):
        self.data.extend(struct.pack('<q', int(val)))

    def write_bytes(self, b):
        self.data.extend(b)

    def write_string(self, s):
        if s is None: s = ""
        b = s.encode('utf-8')
        self.write_int32(len(b) + 1)
        self.write_bytes(b)
        self.write_byte(0)

    def write_array(self, arr, fn):
        self.write_int32(len(arr))
        for item in arr:
            fn(self, item)

    @property
    def length(self):
        return len(self.data)

    def get_data(self):
        return bytes(self.data)