// Retrieve all switch configurations and update web user interface
//
// See: http://demos.jquerymobile.com/1.4.5/listview/
function load_switches() {
    $("#foo").addClass('ui-disabled');
    $.mobile.loading("show", {
        text: "loading",
        textVisible: true
    });

    $.post("/api/switches", function(data) {
        var items = data.length;

        $("#content").empty();
        $("#content").hide();

        $.each(data, function(index, object) {
            $("#content").append("<li>" + object + "</li>");

            $.ajax({
                url: "/api/get/" + object,
                type: "POST",
                async: false,
                timeout: 2000
            }).done(function(data) {
                console.debug(object);
                console.debug(data);
            }).fail(function() {
            }).always(function() {
                items--;
                if (items == 0) {
                    $.mobile.loading("hide");
                    $("#foo").removeClass('ui-disabled');
                }
            });
        });

        $("#content").listview("refresh");
        $("#content").show();
    });
}

// Load web page and set event handlers
$(function() {
    load_switches();

    $("#refresh").click(function(event) {
        event.preventDefault();
        load_switches();
    });
});
