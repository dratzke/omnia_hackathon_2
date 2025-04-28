from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class GetStateRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class StateResponse(_message.Message):
    __slots__ = ("screen", "linear_velocity", "angular_velocity", "relative_angular_velocity", "finished", "results")
    SCREEN_FIELD_NUMBER: _ClassVar[int]
    LINEAR_VELOCITY_FIELD_NUMBER: _ClassVar[int]
    ANGULAR_VELOCITY_FIELD_NUMBER: _ClassVar[int]
    RELATIVE_ANGULAR_VELOCITY_FIELD_NUMBER: _ClassVar[int]
    FINISHED_FIELD_NUMBER: _ClassVar[int]
    RESULTS_FIELD_NUMBER: _ClassVar[int]
    screen: bytes
    linear_velocity: Vec3
    angular_velocity: Vec3
    relative_angular_velocity: Vec3
    finished: bool
    results: _containers.RepeatedCompositeFieldContainer[ResultEntry]
    def __init__(self, screen: _Optional[bytes] = ..., linear_velocity: _Optional[_Union[Vec3, _Mapping]] = ..., angular_velocity: _Optional[_Union[Vec3, _Mapping]] = ..., relative_angular_velocity: _Optional[_Union[Vec3, _Mapping]] = ..., finished: bool = ..., results: _Optional[_Iterable[_Union[ResultEntry, _Mapping]]] = ...) -> None: ...

class ResultEntry(_message.Message):
    __slots__ = ("name", "finish_time", "last_touched_road_id", "last_touched_road_time")
    NAME_FIELD_NUMBER: _ClassVar[int]
    FINISH_TIME_FIELD_NUMBER: _ClassVar[int]
    LAST_TOUCHED_ROAD_ID_FIELD_NUMBER: _ClassVar[int]
    LAST_TOUCHED_ROAD_TIME_FIELD_NUMBER: _ClassVar[int]
    name: str
    finish_time: float
    last_touched_road_id: int
    last_touched_road_time: float
    def __init__(self, name: _Optional[str] = ..., finish_time: _Optional[float] = ..., last_touched_road_id: _Optional[int] = ..., last_touched_road_time: _Optional[float] = ...) -> None: ...

class Vec3(_message.Message):
    __slots__ = ("x", "y", "z")
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    Z_FIELD_NUMBER: _ClassVar[int]
    x: float
    y: float
    z: float
    def __init__(self, x: _Optional[float] = ..., y: _Optional[float] = ..., z: _Optional[float] = ...) -> None: ...

class InputRequest(_message.Message):
    __slots__ = ("forward", "back", "left", "right", "reset")
    FORWARD_FIELD_NUMBER: _ClassVar[int]
    BACK_FIELD_NUMBER: _ClassVar[int]
    LEFT_FIELD_NUMBER: _ClassVar[int]
    RIGHT_FIELD_NUMBER: _ClassVar[int]
    RESET_FIELD_NUMBER: _ClassVar[int]
    forward: bool
    back: bool
    left: bool
    right: bool
    reset: bool
    def __init__(self, forward: bool = ..., back: bool = ..., left: bool = ..., right: bool = ..., reset: bool = ...) -> None: ...

class EmptyResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...
