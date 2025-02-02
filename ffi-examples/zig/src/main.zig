const std = @import("std");

const C = @cImport({
    @cInclude("librustbee.h");
});

const addrs: [2][C.ADDR_LEN]u8 = [_][C.ADDR_LEN]u8{
    [_]u8{ 0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00 },
    [_]u8{ 0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C },
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();

    const alloc = gpa.allocator();

    const argv = try std.process.argsAlloc(alloc);
    defer std.process.argsFree(alloc, argv);

    const power: u8 = if (argv.len > 1) blk: {
        break :blk try std.fmt.parseInt(u8, argv[argv.len - 1], 10);
    } else 1;

    if (!C.launch_daemon()) {
        std.log.err("Failed to launch daemon\n", .{});
        return std.process.exit(1);
    }

    for (addrs) |addr| {
        const device = C.new_device(&addr);
        if (device == null) {
            std.log.err("Failed to create device\n", .{});
            return std.process.exit(1);
        }
        defer C.free_device(device);

        if (!C.try_connect(device)) {
            std.log.err("Failed to connect to the device\n", .{});
            return std.process.exit(1);
        }

        if (!C.set_power(device, power)) {
            std.log.err("Failed to set power\n", .{});
        }
    }
}
